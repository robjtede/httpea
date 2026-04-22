//! Winnow parsers for reusable URI syntax productions from
//! [RFC 3986](https://datatracker.ietf.org/doc/html/rfc3986).

#![cfg_attr(docsrs, feature(doc_cfg))]

use core::{
    net::{Ipv4Addr, Ipv6Addr},
    str,
};

use winnow::{
    combinator::{alt, opt, preceded, repeat, terminated},
    prelude::*,
    stream::{AsChar, Compare, Stream, StreamIsPartial},
    token::{literal, one_of, take_while},
};

/// Parses an absolute path.
///
/// This is a compatibility alias for [`parse_path_absolute`].
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
    parse_path_absolute.parse_next(input)
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
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat::<_, _, (), _, _>(.., parse_query_or_fragment_item)
        .void()
        .parse_next(input)
}

/// Parses a fragment string, without the leading `#`.
///
/// # BNF
///
/// ```text
/// fragment = *( pchar / "/" / "?" )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_fragment;
///
/// let (rest, ()) = parse_fragment.parse_peek(&b"section-2/part?a"[..]).unwrap();
/// assert_eq!(rest, b"");
/// ```
///
/// See:
/// - [RFC 3986 §3.5](https://datatracker.ietf.org/doc/html/rfc3986#section-3.5)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_fragment<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    parse_query.parse_next(input)
}

/// Returns `true` if the given character is in the `unreserved` group.
#[inline]
pub fn is_unreserved(char: impl AsChar) -> bool {
    matches!(
        char.as_char(),
        '0'..='9' | 'A'..='Z' | 'a'..='z' | '-' | '.' | '_' | '~'
    )
}

/// Returns `true` if the given character is in the `sub-delims` group.
#[inline]
pub fn is_sub_delim(char: impl AsChar) -> bool {
    matches!(
        char.as_char(),
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
    )
}

/// Returns `true` if the given character is in the `gen-delims` group.
#[inline]
pub fn is_gen_delim(char: impl AsChar) -> bool {
    matches!(char.as_char(), ':' | '/' | '?' | '#' | '[' | ']' | '@')
}

/// Returns `true` if the given character is in the `reserved` group.
#[inline]
pub fn is_reserved(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_gen_delim(char) || is_sub_delim(char)
}

/// Returns `true` if the given character is an ASCII hexadecimal digit.
#[inline]
pub fn is_hexdig(char: impl AsChar) -> bool {
    char.as_char().is_ascii_hexdigit()
}

/// Returns `true` if the given character is a valid `pchar`.
#[inline]
pub fn is_pchar(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, '%' | ':' | '@')
}

/// Returns `true` if the given character is a valid `segment-nz-nc` character.
#[inline]
pub fn is_pchar_nc(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, '%' | '@')
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

/// Returns `true` if the given character is valid within an `IPvFuture` tail.
#[inline]
pub fn is_ipvfuture_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, ':')
}

/// Returns `true` if the given character is valid within an `IP-literal` body.
#[inline]
pub fn is_ip_literal_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    char.is_ascii_hexdigit() || is_ipvfuture_char(char) || matches!(char, '.')
}

/// Returns `true` if the given slice is a valid `IPv6address` or `IPvFuture` payload.
///
/// See: [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
#[inline]
pub fn is_ip_literal_body(bytes: &[u8]) -> bool {
    parse_ipv6address
        .parse_peek(bytes)
        .is_ok_and(|(rest, ())| rest.is_empty())
        || parse_ipvfuture
            .parse_peek(bytes)
            .is_ok_and(|(rest, ())| rest.is_empty())
}

/// Parses `pct-encoded`.
///
/// # BNF
///
/// ```text
/// pct-encoded = "%" HEXDIG HEXDIG
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_pct_encoded;
///
/// let (rest, ()) = parse_pct_encoded.parse_peek(&b"%20rest"[..]).unwrap();
/// assert_eq!(rest, b"rest");
/// ```
///
/// See:
/// - [RFC 3986 §2.1](https://datatracker.ietf.org/doc/html/rfc3986#section-2.1)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_pct_encoded<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (b'%', take_while(2..=2, is_hexdig))
        .void()
        .parse_next(input)
}

/// Parses `userinfo`.
///
/// # BNF
///
/// ```text
/// userinfo = *( unreserved / pct-encoded / sub-delims / ":" )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_userinfo;
///
/// let (rest, ()) = parse_userinfo.parse_peek(&b"user:pass@example.com"[..]).unwrap();
/// assert_eq!(rest, b"@example.com");
/// ```
///
/// See:
/// - [RFC 3986 §3.2.1](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.1)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_userinfo<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat::<_, _, (), _, _>(.., parse_userinfo_item)
        .void()
        .parse_next(input)
}

/// Parses `host`.
///
/// # BNF
///
/// ```text
/// host = IP-literal / IPv4address / reg-name
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_host;
///
/// let (rest, ()) = parse_host.parse_peek(&b"192.0.2.1:80"[..]).unwrap();
/// assert_eq!(rest, b":80");
/// ```
///
/// See:
/// - [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_host<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((parse_ip_literal, parse_ipv4address, parse_reg_name))
        .void()
        .parse_next(input)
}

/// Parses a `uri-host`.
///
/// This parser preserves the non-empty host behavior used by the HTTP request-target crate.
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
    alt((parse_ip_literal, parse_ipv4address, parse_reg_name_nz))
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
    (b'[', alt((parse_ipv6address, parse_ipvfuture)), b']')
        .void()
        .parse_next(input)
}

/// Parses an `IPvFuture`.
///
/// # BNF
///
/// ```text
/// IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_ipvfuture;
///
/// let (rest, ()) = parse_ipvfuture.parse_peek(&b"vF.token:part]"[..]).unwrap();
/// assert_eq!(rest, b"]");
/// ```
///
/// See:
/// - [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_ipvfuture<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (
        one_of([b'v', b'V']),
        take_while(1.., is_hexdig),
        b'.',
        repeat::<_, _, (), _, _>(1.., parse_ipvfuture_item),
    )
        .void()
        .parse_next(input)
}

/// Parses an `IPv6address`.
///
/// # BNF
///
/// ```text
/// IPv6address =                            6( h16 ":" ) ls32
///             /                       "::" 5( h16 ":" ) ls32
///             / [               h16 ] "::" 4( h16 ":" ) ls32
///             / [ *1( h16 ":" ) h16 ] "::" 3( h16 ":" ) ls32
///             / [ *2( h16 ":" ) h16 ] "::" 2( h16 ":" ) ls32
///             / [ *3( h16 ":" ) h16 ] "::"    h16 ":"   ls32
///             / [ *4( h16 ":" ) h16 ] "::"              ls32
///             / [ *5( h16 ":" ) h16 ] "::"              h16
///             / [ *6( h16 ":" ) h16 ] "::"
/// ```
///
/// See:
/// - [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_ipv6address<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    take_while(1.., is_ipv6address_char)
        .verify(|slice: &I::Slice| parse_ascii::<Ipv6Addr>(slice.as_ref()).is_some())
        .void()
        .parse_next(input)
}

/// Parses an `IPv4address`.
///
/// # BNF
///
/// ```text
/// IPv4address = dec-octet "." dec-octet "." dec-octet "." dec-octet
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_ipv4address;
///
/// let (rest, ()) = parse_ipv4address.parse_peek(&b"127.0.0.1:80"[..]).unwrap();
/// assert_eq!(rest, b":80");
/// ```
///
/// See:
/// - [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_ipv4address<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    take_while(1.., is_ipv4address_char)
        .verify(|slice: &I::Slice| parse_ascii::<Ipv4Addr>(slice.as_ref()).is_some())
        .void()
        .parse_next(input)
}

/// Parses a `dec-octet`.
///
/// # BNF
///
/// ```text
/// dec-octet = DIGIT
///           / %x31-39 DIGIT
///           / "1" 2DIGIT
///           / "2" %x30-34 DIGIT
///           / "25" %x30-35
/// ```
///
/// See:
/// - [RFC 3986 §3.2.2](https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_dec_octet<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    take_while(1..=3, |char: I::Token| char.as_char().is_ascii_digit())
        .verify(|slice: &I::Slice| is_dec_octet(slice.as_ref()))
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
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat::<_, _, (), _, _>(.., parse_reg_name_item)
        .void()
        .parse_next(input)
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
    I::Token: Clone + AsChar,
{
    take_while(.., |char: I::Token| char.as_char().is_ascii_digit())
        .void()
        .parse_next(input)
}

/// Returns `true` if the given character is a valid first character in `scheme`.
#[inline]
pub fn is_scheme_start(char: impl AsChar) -> bool {
    char.is_alpha()
}

/// Returns `true` if the given character is valid in `scheme`.
#[inline]
pub fn is_scheme_char(char: impl AsChar) -> bool {
    let char = char.as_char();
    char.is_ascii_alphanumeric() || matches!(char, '+' | '-' | '.')
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
/// This parser preserves the non-empty host behavior used by the HTTP request-target crate.
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
/// This parser preserves the non-empty host behavior used by the HTTP request-target crate.
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
        opt(terminated(parse_userinfo_nz, b'@')),
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
    repeat::<_, _, (), _, _>(.., (b'/', parse_segment))
        .void()
        .parse_next(input)
}

/// Parses `path-absolute`.
///
/// # BNF
///
/// ```text
/// path-absolute = "/" [ segment-nz *( "/" segment ) ]
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_path_absolute;
///
/// let (rest, ()) = parse_path_absolute.parse_peek(&b"/a/b?x=1"[..]).unwrap();
/// assert_eq!(rest, b"?x=1");
/// ```
///
/// See:
/// - [RFC 3986 §3.3](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_path_absolute<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (
        b'/',
        opt((
            parse_segment_nz,
            repeat::<_, _, (), _, _>(.., (b'/', parse_segment)),
        )),
    )
        .void()
        .parse_next(input)
}

/// Parses `path-noscheme`.
///
/// # BNF
///
/// ```text
/// path-noscheme = segment-nz-nc *( "/" segment )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_path_noscheme;
///
/// let (rest, ()) = parse_path_noscheme.parse_peek(&b"docs/latest?q=1"[..]).unwrap();
/// assert_eq!(rest, b"?q=1");
/// ```
///
/// See:
/// - [RFC 3986 §3.3](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_path_noscheme<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (
        parse_segment_nz_nc,
        repeat::<_, _, (), _, _>(.., (b'/', parse_segment)),
    )
        .void()
        .parse_next(input)
}

/// Parses `path-rootless`.
///
/// # BNF
///
/// ```text
/// path-rootless = segment-nz *( "/" segment )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_path_rootless;
///
/// let (rest, ()) = parse_path_rootless.parse_peek(&b"urn:isbn:0451450523?x"[..]).unwrap();
/// assert_eq!(rest, b"?x");
/// ```
///
/// See:
/// - [RFC 3986 §3.3](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_path_rootless<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (
        parse_segment_nz,
        repeat::<_, _, (), _, _>(.., (b'/', parse_segment)),
    )
        .void()
        .parse_next(input)
}

/// Parses `path-empty`.
///
/// # BNF
///
/// ```text
/// path-empty = 0<pchar>
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_path_empty;
///
/// let (rest, ()) = parse_path_empty.parse_peek(&b"?q=1"[..]).unwrap();
/// assert_eq!(rest, b"?q=1");
/// ```
///
/// See:
/// - [RFC 3986 §3.3](https://datatracker.ietf.org/doc/html/rfc3986#section-3.3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_path_empty<I>(_input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
{
    Ok(())
}

/// Parses `segment`.
///
/// # BNF
///
/// ```text
/// segment = *pchar
/// ```
///
/// See: [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_segment<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat::<_, _, (), _, _>(.., parse_pchar_item)
        .void()
        .parse_next(input)
}

/// Parses `segment-nz`.
///
/// # BNF
///
/// ```text
/// segment-nz = 1*pchar
/// ```
///
/// See: [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_segment_nz<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat::<_, _, (), _, _>(1.., parse_pchar_item)
        .void()
        .parse_next(input)
}

/// Parses `segment-nz-nc`.
///
/// # BNF
///
/// ```text
/// segment-nz-nc = 1*( unreserved / pct-encoded / sub-delims / "@" )
/// ```
///
/// See: [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_segment_nz_nc<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat::<_, _, (), _, _>(1.., parse_pchar_nc_item)
        .void()
        .parse_next(input)
}

/// Parses `hier-part`.
///
/// # BNF
///
/// ```text
/// hier-part = "//" authority path-abempty
///           / path-absolute
///           / path-rootless
///           / path-empty
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_hier_part;
///
/// let (rest, ()) = parse_hier_part.parse_peek(&b"//example.com/a?b"[..]).unwrap();
/// assert_eq!(rest, b"?b");
/// ```
///
/// See:
/// - [RFC 3986 §3](https://datatracker.ietf.org/doc/html/rfc3986#section-3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_hier_part<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((
        ((b'/', b'/'), parse_generic_authority, parse_path_abempty).void(),
        parse_path_absolute,
        parse_path_rootless,
        parse_path_empty,
    ))
    .parse_next(input)
}

/// Parses `relative-part`.
///
/// # BNF
///
/// ```text
/// relative-part = "//" authority path-abempty
///               / path-absolute
///               / path-noscheme
///               / path-empty
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_relative_part;
///
/// let (rest, ()) = parse_relative_part.parse_peek(&b"guides/setup#frag"[..]).unwrap();
/// assert_eq!(rest, b"#frag");
/// ```
///
/// See:
/// - [RFC 3986 §4.2](https://datatracker.ietf.org/doc/html/rfc3986#section-4.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_relative_part<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((
        ((b'/', b'/'), parse_generic_authority, parse_path_abempty).void(),
        parse_path_absolute,
        parse_path_noscheme,
        parse_path_empty,
    ))
    .parse_next(input)
}

/// Parses `absolute-URI`.
///
/// # BNF
///
/// ```text
/// absolute-URI = scheme ":" hier-part [ "?" query ]
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_absolute_uri;
///
/// let (rest, ()) = parse_absolute_uri
///     .parse_peek(&b"mailto:John.Doe@example.com?subject=Hi#frag"[..])
///     .unwrap();
/// assert_eq!(rest, b"#frag");
/// ```
///
/// See:
/// - [RFC 3986 §4.3](https://datatracker.ietf.org/doc/html/rfc3986#section-4.3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_absolute_uri<I>(input: &mut I) -> ModalResult<()>
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

/// Parses `relative-ref`.
///
/// # BNF
///
/// ```text
/// relative-ref = relative-part [ "?" query ] [ "#" fragment ]
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_relative_ref;
///
/// let (rest, ()) = parse_relative_ref
///     .parse_peek(&b"../images/logo.svg?v=2#hero"[..])
///     .unwrap();
/// assert_eq!(rest, b"");
/// ```
///
/// See:
/// - [RFC 3986 §4.2](https://datatracker.ietf.org/doc/html/rfc3986#section-4.2)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_relative_ref<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (
        parse_relative_part,
        opt((b'?', parse_query)),
        opt((b'#', parse_fragment)),
    )
        .void()
        .parse_next(input)
}

/// Parses `URI`.
///
/// # BNF
///
/// ```text
/// URI = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_uri;
///
/// let (rest, ()) = parse_uri
///     .parse_peek(&b"https://example.com/a/b?q=1#frag"[..])
///     .unwrap();
/// assert_eq!(rest, b"");
/// ```
///
/// See:
/// - [RFC 3986 §3](https://datatracker.ietf.org/doc/html/rfc3986#section-3)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_uri<I>(input: &mut I) -> ModalResult<()>
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
        opt((b'#', parse_fragment)),
    )
        .void()
        .parse_next(input)
}

/// Parses `URI-reference`.
///
/// # BNF
///
/// ```text
/// URI-reference = URI / relative-ref
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc3986::parse_uri_reference;
///
/// let (rest, ()) = parse_uri_reference.parse_peek(&b"//example.com/path"[..]).unwrap();
/// assert_eq!(rest, b"");
/// ```
///
/// See:
/// - [RFC 3986 §4.1](https://datatracker.ietf.org/doc/html/rfc3986#section-4.1)
/// - [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_uri_reference<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((parse_uri, parse_relative_ref)).parse_next(input)
}

/// Parses `h16`.
///
/// # BNF
///
/// ```text
/// h16 = 1*4HEXDIG
/// ```
///
/// See: [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_h16<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    take_while(1..=4, is_hexdig).void().parse_next(input)
}

/// Parses `ls32`.
///
/// # BNF
///
/// ```text
/// ls32 = ( h16 ":" h16 ) / IPv4address
/// ```
///
/// See: [RFC 3986 Appendix A](https://datatracker.ietf.org/doc/html/rfc3986#appendix-A)
#[inline]
pub fn parse_ls32<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((parse_ipv4address, (parse_h16, b':', parse_h16).void())).parse_next(input)
}

#[inline]
fn parse_unreserved_item<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    take_while(1..=1, is_unreserved).void().parse_next(input)
}

#[inline]
fn parse_sub_delim_item<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    take_while(1..=1, is_sub_delim).void().parse_next(input)
}

#[inline]
fn parse_reg_name_item<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    alt((
        parse_pct_encoded,
        parse_unreserved_item,
        parse_sub_delim_item,
    ))
    .parse_next(input)
}

#[inline]
fn parse_reg_name_nz<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat::<_, _, (), _, _>(1.., parse_reg_name_item)
        .void()
        .parse_next(input)
}

#[inline]
fn parse_userinfo_item<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    alt((
        parse_pct_encoded,
        parse_unreserved_item,
        parse_sub_delim_item,
        literal(b':').void(),
    ))
    .parse_next(input)
}

#[inline]
fn parse_userinfo_nz<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat::<_, _, (), _, _>(1.., parse_userinfo_item)
        .void()
        .parse_next(input)
}

#[inline]
fn parse_pchar_item<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    alt((
        parse_pct_encoded,
        parse_unreserved_item,
        parse_sub_delim_item,
        one_of([b':', b'@']).void(),
    ))
    .parse_next(input)
}

#[inline]
fn parse_pchar_nc_item<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    alt((
        parse_pct_encoded,
        parse_unreserved_item,
        parse_sub_delim_item,
        literal(b'@').void(),
    ))
    .parse_next(input)
}

#[inline]
fn parse_query_or_fragment_item<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    alt((parse_pchar_item, one_of([b'/', b'?']).void())).parse_next(input)
}

#[inline]
fn parse_ipvfuture_item<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    take_while(1..=1, is_ipvfuture_char)
        .void()
        .parse_next(input)
}

#[inline]
fn parse_generic_authority<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (
        opt(terminated(parse_userinfo, b'@')),
        parse_host,
        opt((b':', parse_port)),
    )
        .void()
        .parse_next(input)
}

#[inline]
fn is_ipv4address_char(char: impl AsChar) -> bool {
    matches!(char.as_char(), '0'..='9' | '.')
}

#[inline]
fn is_ipv6address_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    char.is_ascii_hexdigit() || matches!(char, ':' | '.')
}

#[inline]
fn parse_ascii<T>(bytes: &[u8]) -> Option<T>
where
    T: str::FromStr,
{
    str::from_utf8(bytes).ok()?.parse().ok()
}

#[inline]
fn is_dec_octet(bytes: &[u8]) -> bool {
    matches!(bytes, [b'0'..=b'9'])
        || matches!(bytes, [b'1'..=b'9', b'0'..=b'9'])
        || matches!(bytes, [b'1', b'0'..=b'9', b'0'..=b'9'])
        || matches!(bytes, [b'2', b'0'..=b'4', b'0'..=b'9'])
        || matches!(bytes, [b'2', b'5', b'0'..=b'5'])
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
            assert!(
                matches!(
                    $parser.parse_peek(BStr::new($input)),
                    Ok((remaining, _)) if remaining == BStr::new($remaining)
                ),
                "assertion failed: parser did not leave expected remaining input {:?}: {:?}",
                $remaining,
                $parser.parse_peek(BStr::new($input)),
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
        assert!(is_unreserved('a'));
        assert!(!is_unreserved('/'));
        assert!(is_sub_delim('!'));
        assert!(!is_sub_delim('/'));
        assert!(is_gen_delim('/'));
        assert!(is_reserved('/'));
        assert!(is_hexdig('F'));
        assert!(!is_hexdig('g'));
        assert!(is_pchar('%'));
        assert!(is_pchar_nc('@'));
        assert!(!is_pchar_nc(':'));
        assert!(is_reg_name_char('%'));
        assert!(is_userinfo_char(':'));
        assert!(is_ipvfuture_char(':'));
        assert!(is_ip_literal_char('.'));
    }

    #[test]
    fn validates_ip_literal_bodies() {
        assert!(!is_ip_literal_body(b""));
        assert!(!is_ip_literal_body(b"localhost"));
        assert!(!is_ip_literal_body(b"v1"));
        assert!(is_ip_literal_body(b"::1"));
        assert!(is_ip_literal_body(b"2001:db8::1"));
        assert!(is_ip_literal_body(b"v1.future-host"));
    }

    #[test]
    fn parses_pct_encoded() {
        assert_backtrack!(parse_pct_encoded, b"");
        assert_backtrack!(parse_pct_encoded, b"%");
        assert_backtrack!(parse_pct_encoded, b"%2G");
        assert_partial_incomplete!(parse_pct_encoded, b"%2", Needed::new(1));

        assert_ok_remaining!(parse_pct_encoded, b"%20rest", b"rest");
    }

    #[test]
    fn parses_userinfo_and_reg_name() {
        assert_ok_remaining!(parse_userinfo, b"", b"");
        assert_ok_remaining!(parse_userinfo, b"user:pass@", b"@");
        assert_ok_remaining!(parse_reg_name, b"", b"");
        assert_ok_remaining!(parse_reg_name, b"example.com:80", b":80");
        assert_backtrack!(parse_uri_host, b"");
        assert_ok_remaining!(parse_uri_host, b"example.com:80", b":80");
    }

    #[test]
    fn parses_ipv4_address_and_dec_octet() {
        assert_ok_remaining!(parse_dec_octet, b"0.", b".");
        assert_ok_remaining!(parse_dec_octet, b"255.", b".");
        assert_backtrack!(parse_dec_octet, b"256");
        assert_backtrack!(parse_dec_octet, b"01");

        assert_ok_remaining!(parse_ipv4address, b"127.0.0.1:80", b":80");
        assert_backtrack!(parse_ipv4address, b"256.0.0.1");
        assert_backtrack!(parse_ipv4address, b"127.0.0");
    }

    #[test]
    fn parses_ip_literal_variants() {
        assert_ok_remaining!(parse_h16, b"abcd:", b":");
        assert_ok_remaining!(parse_ls32, b"abcd:ef01", b"");
        assert_ok_remaining!(parse_ls32, b"192.0.2.1", b"");

        assert_ok_remaining!(parse_ipv6address, b"2001:db8::1]", b"]");
        assert_ok_remaining!(parse_ipvfuture, b"vF.token:part]", b"]");
        assert_backtrack!(parse_ipvfuture, b"v.");
        assert_ok_remaining!(parse_ip_literal, b"[::1]:443", b":443");
        assert_ok_remaining!(parse_ip_literal, b"[vF.token:part]/", b"/");
        assert_backtrack!(parse_ip_literal, b"[localhost]");
    }

    #[test]
    fn parses_host_and_authority() {
        assert_ok_remaining!(parse_host, b":80", b":80");
        assert_ok_remaining!(parse_host, b"127.0.0.1:80", b":80");
        assert_ok_remaining!(parse_host, b"[::1]:80", b":80");

        assert_backtrack!(parse_authority, b"");
        assert_ok_remaining!(parse_authority, b"user:pass@example.com:443/path", b"/path");
        assert_ok_remaining!(parse_authority, b"[::1]/path", b"/path");
    }

    #[test]
    fn parses_path_variants() {
        assert_ok_remaining!(parse_segment, b":@", b"");
        assert_backtrack!(parse_segment_nz, b"");
        assert_backtrack!(parse_segment_nz_nc, b":");
        assert_ok_remaining!(parse_segment_nz_nc, b"abc@/rest", b"/rest");

        assert_ok_remaining!(parse_path_abempty, b"/a/b?x=1", b"?x=1");
        assert_ok_remaining!(parse_path_absolute, b"/a/b?x=1", b"?x=1");
        assert_backtrack!(parse_path_absolute, b"foo/bar");
        assert_ok_remaining!(parse_path_noscheme, b"docs/latest?q=1", b"?q=1");
        assert_ok_remaining!(parse_path_noscheme, b"urn:ietf", b":ietf");
        assert_ok_remaining!(parse_path_rootless, b"urn:ietf:rfc:3986#frag", b"#frag");
        assert_ok_remaining!(parse_path_empty, b"?q=1", b"?q=1");
        assert_ok_remaining!(parse_path, b"/foo%20bar?baz", b"?baz");
        assert_ok_remaining!(parse_path, b"/foo%", b"%");
    }

    #[test]
    fn parses_query_and_fragment() {
        assert_ok_remaining!(parse_query, b"foo=bar/baz?x#frag", b"#frag");
        assert_ok_remaining!(parse_query, b"foo%", b"%");
        assert_ok_remaining!(parse_fragment, b"section-2/part?a", b"");
    }

    #[test]
    fn parses_hier_part_and_relative_part() {
        assert_ok_remaining!(parse_hier_part, b"//example.com/a?b", b"?b");
        assert_ok_remaining!(parse_hier_part, b"/a/b?c", b"?c");
        assert_ok_remaining!(parse_hier_part, b"mailto:John.Doe@example.com", b"");

        assert_ok_remaining!(parse_relative_part, b"//example.com/a?b", b"?b");
        assert_ok_remaining!(parse_relative_part, b"/a/b?b", b"?b");
        assert_ok_remaining!(parse_relative_part, b"guides/setup#frag", b"#frag");
    }

    #[test]
    fn parses_uri_forms() {
        assert_partial_incomplete!(parse_scheme, b"", Needed::new(1));

        assert_ok_remaining!(
            parse_absolute_uri,
            b"mailto:John.Doe@example.com?subject=Hi#frag",
            b"#frag"
        );
        assert_ok_remaining!(parse_relative_ref, b"../images/logo.svg?v=2#hero", b"");
        assert_ok_remaining!(parse_uri, b"https://example.com/a/b?q=1#frag", b"");
        assert_ok_remaining!(parse_uri_reference, b"//example.com/path", b"");
        assert_ok_remaining!(parse_uri_reference, b"urn:ietf:rfc:3986", b"");

        assert_backtrack!(parse_uri, b"//example.com/path");
        assert_ok_remaining!(parse_uri_reference, b"http://example.com/%", b"%");
    }
}
