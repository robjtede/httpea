//! Winnow parsers for reusable URI syntax productions from
//! [RFC 3986](https://datatracker.ietf.org/doc/html/rfc3986).

#![cfg_attr(docsrs, feature(doc_cfg))]

use winnow::{
    ascii::digit1,
    combinator::{alt, delimited, opt, peek, preceded, repeat},
    prelude::*,
    stream::{AsChar, Compare, Stream, StreamIsPartial},
    token::{one_of, take_while},
};

/// Parses an absolute path.
///
/// # BNF
///
/// ```text
/// absolute-path = 1*( "/" segment )
/// segment       = *pchar
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_path;
///
/// let (rest, ()) = parse_path.parse_peek(&b"/a/b?c"[..]).unwrap();
/// assert_eq!(rest, b"?c");
/// ```
///
/// See:
/// - [RFC 3986 §3.3](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_path<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    peek(b'/').parse_next(input)?;

    repeat(1.., (b'/', take_while(.., is_pchar)))
        .map(|()| ())
        .void()
        .parse_next(input)
}

/// Parses a query string, without the leading `?`.
///
/// # BNF
///
/// ```text
/// query = *( pchar / "/" / "?" )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_query;
///
/// let (rest, ()) = parse_query.parse_peek(&b"foo=bar/baz?x#frag"[..]).unwrap();
/// assert_eq!(rest, b"#frag");
/// ```
///
/// See:
/// - [RFC 3986 §3.4](https://datatracker.ietf.org/doc/html/rfc3986#section-3.4)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_query<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    repeat(
        ..,
        one_of((
            is_pchar,
            #[allow(clippy::byte_char_slices)]
            [b'/', b'?'],
        )),
    )
    .map(|()| ())
    .void()
    .parse_next(input)
}

/// Returns `true` if the given character is in the `unreserved` group.
#[inline]
pub fn is_unreserved(char: char) -> bool {
    matches!(char, '0'..='9' | 'A'..='Z' | 'a'..='z' | '-' | '.' | '_' | '~')
}

/// Returns `true` if the given character is in the `sub-delims` group.
#[inline]
pub fn is_sub_delim(char: char) -> bool {
    matches!(
        char,
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
    )
}

/// Returns `true` if the given character is a valid `pchar`.
#[inline]
pub fn is_pchar(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, '%' | ':' | '@')
}

/// Returns `true` if the given character is valid in `reg-name`.
#[inline]
pub fn is_reg_name_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, '%')
}

/// Returns `true` if the given character is valid in `userinfo`.
#[inline]
pub fn is_userinfo_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, '%' | ':')
}

/// Returns `true` if the given character is valid within an `IP-literal` body.
#[inline]
pub fn is_ip_literal_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, ':')
}

/// Returns `true` if the given slice looks like an `IPv6address` or `IPvFuture` payload.
///
/// See: [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
#[inline]
pub fn is_ip_literal_body(bytes: &[u8]) -> bool {
    bytes.contains(&b':') || matches!(bytes.first(), Some(b'v' | b'V')) && bytes.contains(&b'.')
}

/// Parses a `uri-host`.
///
/// # BNF
///
/// ```text
/// host = IP-literal / IPv4address / reg-name
/// uri-host = <HTTP request-target use of URI host grammar>
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_uri_host;
///
/// let (rest, ()) = parse_uri_host.parse_peek(&b"[::1]:443"[..]).unwrap();
/// assert_eq!(rest, b":443");
/// ```
///
/// See:
/// - [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_uri_host<I>(input: &mut I) -> ModalResult<()>
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
/// # BNF
///
/// ```text
/// IP-literal = "[" ( IPv6address / IPvFuture ) "]"
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_ip_literal;
///
/// let (rest, ()) = parse_ip_literal.parse_peek(&b"[2001:db8::1]/"[..]).unwrap();
/// assert_eq!(rest, b"/");
/// ```
///
/// See:
/// - [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_ip_literal<I>(input: &mut I) -> ModalResult<()>
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
/// # BNF
///
/// ```text
/// reg-name = *( unreserved / pct-encoded / sub-delims )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_reg_name;
///
/// let (rest, ()) = parse_reg_name.parse_peek(&b"example.com:443"[..]).unwrap();
/// assert_eq!(rest, b":443");
/// ```
///
/// See:
/// - [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_reg_name<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    take_while(1.., is_reg_name_char).void().parse_next(input)
}

/// Parses a `port`.
///
/// # BNF
///
/// ```text
/// port = *DIGIT
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_port;
///
/// let (rest, ()) = parse_port.parse_peek(&b"8443/path"[..]).unwrap();
/// assert_eq!(rest, b"/path");
/// ```
///
/// See:
/// - [RFC 3986 §3.2.3](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_port<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: AsChar,
{
    digit1.void().parse_next(input)
}

/// Returns `true` if the given character is a valid first character in `scheme`.
pub fn is_scheme_start(char: impl AsChar) -> bool {
    char.as_char().is_ascii_alphabetic()
}

/// Returns `true` if the given character is valid in `scheme`.
pub fn is_scheme_char(char: impl AsChar) -> bool {
    matches!(char.as_char(), '0'..='9' | 'A'..='Z' | 'a'..='z' | '+' | '-' | '.')
}

/// Parses `scheme`.
///
/// # BNF
///
/// ```text
/// scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_scheme;
///
/// let (rest, ()) = parse_scheme.parse_peek(&b"https://example.com"[..]).unwrap();
/// assert_eq!(rest, b"://example.com");
/// ```
///
/// See:
/// - [RFC 3986 §3.1](https://datatracker.ietf.org/doc/html/rfc3986#section-3.1)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_scheme<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    preceded(one_of(is_scheme_start), take_while(.., is_scheme_char))
        .void()
        .parse_next(input)
}

/// Parses `authority`.
///
/// # BNF
///
/// ```text
/// authority = [ userinfo "@" ] host [ ":" port ]
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_authority;
///
/// let (rest, ()) = parse_authority
///     .parse_peek(&b"user:pass@example.com:443/path"[..])
///     .unwrap();
/// assert_eq!(rest, b"/path");
/// ```
///
/// See:
/// - [RFC 3986 §3.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_authority<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    parse_authority_parts.void().parse_next(input)
}

/// Parses `authority` into `userinfo`, `host`, and optional `port`.
///
/// # BNF
///
/// ```text
/// authority = [ userinfo "@" ] host [ ":" port ]
/// userinfo  = *( unreserved / pct-encoded / sub-delims / ":" )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_authority_parts;
///
/// let (rest, ()) = parse_authority_parts
///     .parse_peek(&b"user:pass@example.com:443/path"[..])
///     .unwrap();
/// assert_eq!(rest, b"/path");
/// ```
///
/// See:
/// - [RFC 3986 §3.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_authority_parts<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (
        opt((take_while(1.., is_userinfo_char), b'@')),
        parse_uri_host,
        opt((b':', parse_port)),
    )
        .void()
        .parse_next(input)
}

/// Parses `path-abempty`.
///
/// # BNF
///
/// ```text
/// path-abempty = *( "/" segment )
/// segment      = *pchar
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_path_abempty;
///
/// let (rest, ()) = parse_path_abempty.parse_peek(&b"/a/b?x=1"[..]).unwrap();
/// assert_eq!(rest, b"?x=1");
/// ```
///
/// See:
/// - [RFC 3986 §3.3](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_path_abempty<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat(.., (b'/', take_while(.., is_pchar)))
        .map(|()| ())
        .void()
        .parse_next(input)
}
