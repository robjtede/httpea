//! HTTP/1.1 request-target (RFC 9112) parser.

// #![no_std]
#![expect(dead_code)]

extern crate alloc;

use alloc::string::String;

use winnow::{
    combinator::{alt, fail, opt, peek, repeat, todo},
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
    Absolute(String),

    /// ```plain
    /// authority-form = uri-host ":" port
    /// CONNECT www.example.com:80 HTTP/1.1
    /// ```
    Authority(String),

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
            parse_origin_form
                .take()
                .map(|input| RequestTarget::Origin(input)),
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
fn parse_path<'a, I>(input: &'a mut I) -> ModalResult<()>
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

/// Returns `true` if the given character is a valid "pchar" (path character).
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
fn is_pchar(char: impl AsChar) -> bool {
    match char.as_char() {
        // unreserved
        '0'..='9' | 'A'..='Z' | 'a'..='z' => true,

        // unreserved
        '-' | '.' | '_' | '~' => true,

        // pct-encoded
        '%' => true, // HEXDIG are included in unreserved; we do not validate hex escape seqences

        // sub-delims
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' => true,

        // pchar literals
        ':' | '@' => true,

        _ => false,
    }
}

/// # Request Line Examples
///
/// ```plain
///
/// ```
fn parse_authority_form<I>(input: &mut I) -> ModalResult<()>
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

#[cfg(test)]
mod tests {
    use winnow::{
        BStr, Partial,
        error::{ErrMode, Needed},
    };

    use super::*;

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
