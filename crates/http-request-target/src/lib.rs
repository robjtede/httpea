//! HTTP/1.1 request-target (RFC 9112) parser.

// #![no_std]
use winnow::{
    combinator::{alt, delimited, empty, fail, opt, peek, repeat},
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

/// Returns `true` if the given character is valid in `userinfo`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.1>
fn is_userinfo_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char)
        || is_sub_delim(char)
        // pct-encoded
        || matches!(char, '%') // HEXDIG are included in `unreserved`; we do not validate hex escape sequences
        || matches!(char, ':')
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
/// GET http://www.example.org/pub/WWW/TheProject.html HTTP/1.1
/// ```
fn parse_absolute_form<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (
        parse_scheme,
        b':',
        parse_hier_part,
        opt((b'?', parse_query)),
    )
        .void()
        .parse_next(input)
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
    alt((
        parse_ip_literal,
        // IPv4 addresses are valid in `reg-name` context
        parse_reg_name,
    ))
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

/// Returns `true` if the given character is a valid first character in `scheme`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.1>
fn is_scheme_start(char: impl AsChar) -> bool {
    matches!(char.as_char(), 'A'..='Z' | 'a'..='z')
}

/// Returns `true` if the given character is valid in `scheme`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.1>
fn is_scheme_char(char: impl AsChar) -> bool {
    matches!(char.as_char(), '0'..='9' | 'A'..='Z' | 'a'..='z' | '+' | '-' | '.')
}

/// Parses `scheme`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.1>
fn parse_scheme<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    (one_of(is_scheme_start), take_while(.., is_scheme_char))
        .void()
        .parse_next(input)
}

/// Parses `hier-part`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3>
fn parse_hier_part<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((
        ((b'/', b'/'), parse_authority, parse_path_abempty).void(),
        parse_path_absolute,
        parse_path_rootless,
        empty.void(),
    ))
    .parse_next(input)
}

/// Parses `authority`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.2>
fn parse_authority<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (
        opt((take_while(1.., is_userinfo_char), b'@')),
        parse_uri_host,
        opt((
            b':',
            take_while(1.., |char: I::Token| matches!(char.as_char(), '0'..='9')),
        )),
    )
        .void()
        .parse_next(input)
}

/// Parses `path-abempty`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
fn parse_path_abempty<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat(.., (b'/', take_while(.., is_pchar)))
        .map(|()| ())
        .void()
        .parse_next(input)
}

/// Parses `path-absolute`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
fn parse_path_absolute<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (
        b'/',
        opt((
            take_while(1.., is_pchar),
            repeat(.., (b'/', take_while(.., is_pchar)))
                .map(|()| ())
                .void(),
        )),
    )
        .void()
        .parse_next(input)
}

/// Parses `path-rootless`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
fn parse_path_rootless<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (
        take_while(1.., is_pchar),
        repeat(.., (b'/', take_while(.., is_pchar)))
            .map(|()| ())
            .void(),
    )
        .void()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::{
        BStr, Partial,
        error::{ErrMode, Needed},
    };

    use super::*;

    macro_rules! assert_backtrack {
        ($parser:expr, $input:expr $(,)?) => {
            assert!(
                matches!(
                    $parser.parse_peek(BStr::new($input)),
                    Err(ErrMode::Backtrack(_))
                ),
                "assertion failed: parser did not backtrack for input {:?}: {:?}",
                $input,
                $parser.parse_peek(BStr::new($input)),
            );
        };
    }

    macro_rules! assert_ok_remaining {
        ($parser:expr, $input:expr, $remaining:expr $(,)?) => {
            assert_eq!(
                $parser.parse_peek(BStr::new($input)),
                Ok((BStr::new($remaining), ())),
            );
        };
    }

    macro_rules! assert_partial_incomplete {
        ($parser:expr, $input:expr, $needed:expr $(,)?) => {
            assert_eq!(
                $parser.parse_peek(Partial::new(BStr::new($input))),
                Err(ErrMode::Incomplete($needed)),
            );
        };
    }

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
        assert_backtrack!(parse_reg_name, b"");
        assert_backtrack!(parse_reg_name, b"@localhost");

        assert_ok_remaining!(parse_reg_name, b"localhost", b"");
        assert_ok_remaining!(parse_reg_name, b"example.com:80", b":80");
        assert_ok_remaining!(parse_reg_name, b"xn--hllo-bpa.example", b"");
    }

    #[test]
    fn parses_ip_literal() {
        assert_backtrack!(parse_ip_literal, b"");
        assert_backtrack!(parse_ip_literal, b"[localhost]");
        assert_backtrack!(parse_ip_literal, b"[::1");

        assert_ok_remaining!(parse_ip_literal, b"[::1]", b"");
        assert_ok_remaining!(parse_ip_literal, b"[2001:db8::1]:443", b":443");
        assert_ok_remaining!(parse_ip_literal, b"[v1.future-host]", b"");
    }

    #[test]
    fn parses_uri_host() {
        assert_backtrack!(parse_uri_host, b"");
        assert_backtrack!(parse_uri_host, b"@localhost:80");

        assert_ok_remaining!(parse_uri_host, b"localhost:80", b":80");
        assert_ok_remaining!(parse_uri_host, b"127.0.0.1:80", b":80");
        assert_ok_remaining!(parse_uri_host, b"[::1]:80", b":80");
    }

    #[test]
    fn parses_scheme() {
        assert_backtrack!(parse_scheme, b"");
        assert_backtrack!(parse_scheme, b"1http");
        assert_partial_incomplete!(parse_scheme, b"", Needed::new(1));

        assert_ok_remaining!(parse_scheme, b"http", b"");
        assert_ok_remaining!(parse_scheme, b"https:", b":");
        assert_ok_remaining!(parse_scheme, b"http+unix:", b":");
    }

    #[test]
    fn parses_authority() {
        assert_backtrack!(parse_authority, b"");
        assert_backtrack!(parse_authority, b"@localhost");
        assert_partial_incomplete!(parse_authority, b"", Needed::new(1));

        assert_ok_remaining!(parse_authority, b"127.0.0.1:80", b"");
        assert_ok_remaining!(parse_authority, b"user:pass@localhost:3000", b"");
        assert_ok_remaining!(parse_authority, b"[::1]/path", b"/path");
    }

    #[test]
    fn parses_path_abempty() {
        assert_ok_remaining!(parse_path_abempty, b"", b"");
        assert_ok_remaining!(parse_path_abempty, b"/", b"");
        assert_ok_remaining!(parse_path_abempty, b"/foo/bar", b"");
        assert_ok_remaining!(parse_path_abempty, b"?foo=bar", b"?foo=bar");
    }

    #[test]
    fn parses_path_absolute() {
        assert_backtrack!(parse_path_absolute, b"");
        assert_backtrack!(parse_path_absolute, b"foo/bar");
        assert_partial_incomplete!(parse_path_absolute, b"", Needed::Unknown);

        assert_ok_remaining!(parse_path_absolute, b"/", b"");
        assert_ok_remaining!(parse_path_absolute, b"/foo/bar", b"");
        assert_ok_remaining!(parse_path_absolute, b"/foo?bar", b"?bar");
    }

    #[test]
    fn parses_path_rootless() {
        assert_backtrack!(parse_path_rootless, b"");
        assert_backtrack!(parse_path_rootless, b"/foo");
        assert_partial_incomplete!(parse_path_rootless, b"", Needed::new(1));

        assert_ok_remaining!(parse_path_rootless, b"foo", b"");
        assert_ok_remaining!(parse_path_rootless, b"foo/bar", b"");
        assert_ok_remaining!(parse_path_rootless, b"foo?bar", b"?bar");
    }

    #[test]
    fn parses_hier_part() {
        assert_partial_incomplete!(parse_hier_part, b"//", Needed::new(1));

        assert_ok_remaining!(parse_hier_part, b"", b"");
        assert_ok_remaining!(parse_hier_part, b"//127.0.0.1:80/path", b"");
        assert_ok_remaining!(parse_hier_part, b"/foo/bar", b"");
        assert_ok_remaining!(parse_hier_part, b"foo/bar", b"");
    }

    #[test]
    fn parses_path() {
        assert_backtrack!(parse_path, b"");
        assert_partial_incomplete!(parse_path, b"", Needed::Unknown);
        assert_backtrack!(parse_path, b"=");

        assert_ok_remaining!(parse_path, b"/foo", b"");
        assert_ok_remaining!(parse_path, b"/foo/bar", b"");

        // parser assumes it won't receive a query but doesn't fail
        assert_ok_remaining!(parse_path, b"/foo/bar?baz", b"?baz");
    }

    #[test]
    fn parses_query() {
        assert_ok_remaining!(parse_query, b"", b"");
        assert_ok_remaining!(parse_query, b"=", b"");
        assert_ok_remaining!(parse_query, b"foo=bar", b"");
        assert_ok_remaining!(parse_query, b"foo=bar&baz", b"");
    }

    #[test]
    fn parses_authority_form() {
        assert_backtrack!(parse_authority_form, b"");
        assert_partial_incomplete!(parse_authority_form, b"", Needed::Unknown);

        assert_backtrack!(parse_authority_form, b"localhost");
        assert_backtrack!(parse_authority_form, b"user@localhost:3000");
        assert_backtrack!(parse_authority_form, b"[::1]");

        assert_ok_remaining!(parse_authority_form, b"localhost:3000", b"");
        assert_ok_remaining!(parse_authority_form, b"127.0.0.1:80", b"");
        assert_ok_remaining!(parse_authority_form, b"[::1]:443", b"");
    }

    #[test]
    fn parses_absolute_form() {
        assert_backtrack!(parse_absolute_form, b"");
        assert_backtrack!(parse_absolute_form, b"/foo");
        assert_partial_incomplete!(parse_absolute_form, b"", Needed::new(1));

        assert_ok_remaining!(parse_absolute_form, b"http://127.0.0.1:61761/chunks", b"");
        assert_ok_remaining!(parse_absolute_form, b"https://127.0.0.1:61761", b"");
        assert_ok_remaining!(parse_absolute_form, b"http://127.0.0.1?foo=bar", b"");
    }

    #[test]
    fn parses_asterisk() {
        assert_backtrack!(parse_asterisk, b"");
        assert_partial_incomplete!(parse_asterisk, b"", Needed::Unknown);

        assert_ok_remaining!(parse_asterisk, b"*", b"");
        assert_ok_remaining!(parse_asterisk, b"**", b"*");
    }
}
