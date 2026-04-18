//! HTTP/1.1 request-target (RFC 9112) parser.

// #![no_std]
use winnow::{
    combinator::{alt, delimited, fail, opt, peek, repeat, todo},
    error::ContextError,
    prelude::*,
    stream::{AsChar, Compare, Stream, StreamIsPartial},
    token::{literal, one_of, take_while},
};

mod error;

// use crate::error::ParseRequestTargetError;

/// See <https://datatracker.ietf.org/doc/html/rfc9112#name-request-target>.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTarget<'a> {
    /// Origin form.
    Origin(&'a [u8]),

    /// ```plain
    /// absolute-form = absolute-URI
    /// GET http://www.example.org/pub/WWW/TheProject.html HTTP/1.1
    /// ```
    Absolute(&'a [u8]),

    /// ```plain
    /// authority-form = uri-host ":" port
    /// CONNECT www.example.com:80 HTTP/1.1
    /// ```
    Authority(&'a [u8]),

    /// Asterisk form.
    ///
    /// ```plain
    /// asterisk-form = "*"
    /// OPTIONS * HTTP/1.1
    /// ```
    Asterisk,
}

impl<'a> RequestTarget<'a> {
    /// Parse request target from slice.
    pub fn try_from_slice(
        input: &'a [u8],
    ) -> Result<Self, winnow::error::ParseError<&'a [u8], ContextError>> {
        // #[cfg(any(debug_assertions, test))]
        // let input = winnow::BStr::new(input);

        alt((
            parse_asterisk.value(RequestTarget::Asterisk),
            parse_origin_form.take().map(RequestTarget::Origin),
            parse_authority_form.take().map(RequestTarget::Authority),
            parse_absolute_form.take().map(RequestTarget::Absolute),
            fail,
        ))
        .parse(input)
    }
}

/// # Request Line Examples
///
/// ```plain
/// GET /where?q=now HTTP/1.1
/// ```
///
/// # BNF
///
/// ```plain
/// origin-form   = absolute-path [ "?" query ]
/// absolute-path = 1*( "/" segment )
/// segment       = *pchar
/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
/// unreserved    = ALPHA / DIGIT / "-" / "." / "_" / "~"
/// pct-encoded   = "%" HEXDIG HEXDIG
/// sub-delims    = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
/// query         = *( pchar / "/" / "?" )
/// ```
fn parse_origin_form<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (parse_path, opt((b'?', parse_query)))
        .void()
        .parse_next(input)
}

/// Parses a path.
///
/// Assumes entire input is a path, starting with a `/`, and does not include a query or fragment.
///
/// See:
/// - <https://datatracker.ietf.org/doc/html/rfc9112#name-syntax-notation>
/// - <https://datatracker.ietf.org/doc/html/rfc9110#name-uri-references>
/// - <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
fn parse_path<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    peek(b'/').parse_next(input)?;

    // absolute-path
    repeat(1.., (b'/', take_while(.., is_pchar)))
        .map(|()| ())
        .void()
        .parse_next(input)
}

/// Parses a query string.
///
/// Assumes entire input is only a query string without preceding `?` and without a rogue, trailing
/// fragement (i.e. `#`).
///
/// See:
/// - <https://datatracker.ietf.org/doc/html/rfc3986#section-3.4>
/// - <https://datatracker.ietf.org/doc/html/rfc3986#appendix-A>
fn parse_query<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    repeat(
        ..,
        one_of((
            is_pchar,
            // query literals
            [b'/', b'?'],
        )),
    )
    .map(|()| ())
    .void()
    .parse_next(input)
}

/// Returns `true` if the given character is in the `unreserved` group.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#appendix-A>
fn is_unreserved(char: char) -> bool {
    matches!(char, '0'..='9' | 'A'..='Z' | 'a'..='z' | '-' | '.' | '_' | '~')
}

/// Returns `true` if the given character is in the `sub-delims` group.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#appendix-A>
fn is_sub_delim(char: char) -> bool {
    matches!(
        char,
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
    )
}

/// Returns `true` if the given character is a valid `pchar`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
fn is_pchar(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char)
        || is_sub_delim(char)
        // pct-encoded
        || matches!(char, '%') // HEXDIG are included in `unreserved`; we do not validate hex escape sequences
        // pchar literals
        || matches!(char, ':' | '@')
}

/// Returns `true` if the given character is valid in `reg-name`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2>
fn is_reg_name_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char)
        || is_sub_delim(char)
        // pct-encoded
        || matches!(char, '%') // HEXDIG are included in `unreserved`; we do not validate hex escape sequences
}

/// Returns `true` if the given character is valid within an `IP-literal` body.
///
/// This covers the character groups referenced by the `IPv6address` and `IPvFuture`
/// productions from RFC 3986.
fn is_ip_literal_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char)
        || is_sub_delim(char)
        // IPv6address / IPvFuture literals
        || matches!(char, ':')
}

/// Returns `true` if the given slice looks like an `IPv6address` or `IPvFuture` payload.
fn is_ip_literal_body(bytes: &[u8]) -> bool {
    bytes.contains(&b':') || matches!(bytes.first(), Some(b'v' | b'V')) && bytes.contains(&b'.')
}

/// # Request Line Examples
///
/// ```plain
/// CONNECT www.example.com:80 HTTP/1.1
/// ```
fn parse_authority_form<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (
        parse_uri_host,
        b':',
        take_while(1.., |char: I::Token| matches!(char.as_char(), '0'..='9')),
    )
        .void()
        .parse_next(input)
}

/// # Request Line Examples
///
/// ```plain
///
/// ```
fn parse_absolute_form<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream,
{
    todo(input)
}

/// # Request Line Examples
///
/// ```plain
/// OPTIONS * HTTP/1.1
/// ```
fn parse_asterisk<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
{
    literal(b'*').void().parse_next(input)
}

/// Parses a `uri-host`.
///
/// RFC 9112 defines `authority-form = uri-host ":" port` and references the URI grammar for the
/// host production.
fn parse_uri_host<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((parse_ip_literal, parse_reg_name))
        .void()
        .parse_next(input)
}

/// Parses an `IP-literal`.
///
/// We enforce the surrounding brackets and restrict the inner character set to the `IPv6address` /
/// `IPvFuture` productions, while leaving the detailed numeric validation to a future pass.
fn parse_ip_literal<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    delimited(
        b'[',
        take_while(1.., is_ip_literal_char)
            .verify(|slice: &I::Slice| is_ip_literal_body(slice.as_ref())),
        b']',
    )
    .void()
    .parse_next(input)
}

/// Parses a `reg-name`.
///
/// RFC 3986 permits an empty `reg-name`, but `authority-form` requires a concrete `uri-host`, so
/// we require at least one character here.
fn parse_reg_name<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    take_while(1.., is_reg_name_char).void().parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::{
        BStr, Partial,
        error::{ErrMode, Needed},
    };

    use super::*;

    #[test]
    fn validates_char_groups() {
        assert!(!is_unreserved('/'));
        assert!(is_unreserved('a'));
        assert!(is_unreserved('Z'));
        assert!(is_unreserved('0'));
        assert!(is_unreserved('~'));

        assert!(!is_sub_delim(':'));
        assert!(is_sub_delim('!'));
        assert!(is_sub_delim('='));

        assert!(!is_pchar(b'/'));
        assert!(is_pchar(b'='));
        assert!(is_pchar(b'%'));
        assert!(is_pchar(b':'));
        assert!(is_pchar(b'@'));

        assert!(!is_reg_name_char(b':'));
        assert!(!is_reg_name_char(b'@'));
        assert!(is_reg_name_char(b'%'));
        assert!(is_reg_name_char(b'.'));

        assert!(!is_ip_literal_char(b'['));
        assert!(!is_ip_literal_char(b'/'));
        assert!(is_ip_literal_char(b':'));
        assert!(is_ip_literal_char(b'v'));
        assert!(is_ip_literal_char(b'.'));
    }

    #[test]
    fn validates_ip_literal_bodies() {
        assert!(!is_ip_literal_body(b""));
        assert!(!is_ip_literal_body(b"localhost"));
        assert!(!is_ip_literal_body(b"v1"));
        assert!(!is_ip_literal_body(b"v1/"));

        assert!(is_ip_literal_body(b"::1"));
        assert!(is_ip_literal_body(b"2001:db8::1"));
        assert!(is_ip_literal_body(b"v1.future-host"));
        assert!(is_ip_literal_body(b"Vf.token:more"));
    }

    #[test]
    fn parses_reg_name() {
        match parse_reg_name.parse_peek(BStr::new(b"")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        match parse_reg_name.parse_peek(BStr::new(b"@localhost")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }

        assert_eq!(
            parse_reg_name.parse_peek(BStr::new(b"localhost")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_reg_name.parse_peek(BStr::new(b"example.com:80")),
            Ok((BStr::new(b":80"), ())),
        );
        assert_eq!(
            parse_reg_name.parse_peek(BStr::new(b"xn--hllo-bpa.example")),
            Ok((BStr::new(b""), ())),
        );
    }

    #[test]
    fn parses_ip_literal() {
        match parse_ip_literal.parse_peek(BStr::new(b"")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        match parse_ip_literal.parse_peek(BStr::new(b"[localhost]")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        match parse_ip_literal.parse_peek(BStr::new(b"[::1")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }

        assert_eq!(
            parse_ip_literal.parse_peek(BStr::new(b"[::1]")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_ip_literal.parse_peek(BStr::new(b"[2001:db8::1]:443")),
            Ok((BStr::new(b":443"), ())),
        );
        assert_eq!(
            parse_ip_literal.parse_peek(BStr::new(b"[v1.future-host]")),
            Ok((BStr::new(b""), ())),
        );
    }

    #[test]
    fn parses_uri_host() {
        match parse_uri_host.parse_peek(BStr::new(b"")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        match parse_uri_host.parse_peek(BStr::new(b"@localhost:80")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }

        assert_eq!(
            parse_uri_host.parse_peek(BStr::new(b"localhost:80")),
            Ok((BStr::new(b":80"), ())),
        );
        assert_eq!(
            parse_uri_host.parse_peek(BStr::new(b"127.0.0.1:80")),
            Ok((BStr::new(b":80"), ())),
        );
        assert_eq!(
            parse_uri_host.parse_peek(BStr::new(b"[::1]:80")),
            Ok((BStr::new(b":80"), ())),
        );
    }

    #[test]
    fn parses_path() {
        match parse_path.parse_peek(BStr::new(b"")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        assert_eq!(
            parse_path.parse_peek(Partial::new(BStr::new(b""))),
            Err(ErrMode::Incomplete(Needed::Unknown)),
        );

        match parse_path.parse_peek(BStr::new(b"=")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }

        assert_eq!(
            parse_path.parse_peek(BStr::new(b"/foo")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_path.parse_peek(BStr::new(b"/foo/bar")),
            Ok((BStr::new(b""), ())),
        );

        // parser assumes it won't receive a query but doesn't fail
        assert_eq!(
            parse_path.parse_peek(BStr::new(b"/foo/bar?baz")),
            Ok((BStr::new(b"?baz"), ())),
        );
    }

    #[test]
    fn parses_query() {
        // `query = *( pchar / "/" / "?" )`, so only success cases exist here.
        assert_eq!(
            parse_query.parse_peek(BStr::new(b"")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_query.parse_peek(BStr::new(b"=")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_query.parse_peek(BStr::new(b"foo=bar")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_query.parse_peek(BStr::new(b"foo=bar&baz")),
            Ok((BStr::new(b""), ())),
        );
    }

    #[test]
    fn parses_authority_form() {
        match parse_authority_form.parse_peek(BStr::new(b"")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        assert_eq!(
            parse_authority_form.parse_peek(Partial::new(BStr::new(b""))),
            Err(ErrMode::Incomplete(Needed::Unknown)),
        );
        match parse_authority_form.parse_peek(BStr::new(b"localhost")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        match parse_authority_form.parse_peek(BStr::new(b"user@localhost:3000")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        match parse_authority_form.parse_peek(BStr::new(b"[::1]")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }

        assert_eq!(
            parse_authority_form.parse_peek(BStr::new(b"localhost:3000")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_authority_form.parse_peek(BStr::new(b"127.0.0.1:80")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_authority_form.parse_peek(BStr::new(b"[::1]:443")),
            Ok((BStr::new(b""), ())),
        );
    }

    #[test]
    fn parses_asterisk() {
        match parse_asterisk.parse_peek(BStr::new(b"")) {
            Err(ErrMode::Backtrack(_)) => {}
            result => panic!("Unexpected result: {result:?}"),
        }
        assert_eq!(
            parse_asterisk.parse_peek(Partial::new(BStr::new(b""))),
            Err(ErrMode::Incomplete(Needed::Unknown)),
        );

        assert_eq!(
            parse_asterisk.parse_peek(BStr::new(b"*")),
            Ok((BStr::new(b""), ())),
        );
        assert_eq!(
            parse_asterisk.parse_peek(BStr::new(b"**")),
            Ok((BStr::new(b"*"), ())),
        );
    }
}
