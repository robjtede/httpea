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
#[inline]
pub fn parse_query<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    repeat(.., one_of((is_pchar, [b'/', b'?'])))
        .map(|()| ())
        .void()
        .parse_next(input)
}

/// Returns `true` if the given character is in the `unreserved` group.
pub fn is_unreserved(char: char) -> bool {
    matches!(char, '0'..='9' | 'A'..='Z' | 'a'..='z' | '-' | '.' | '_' | '~')
}

/// Returns `true` if the given character is in the `sub-delims` group.
pub fn is_sub_delim(char: char) -> bool {
    matches!(
        char,
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
    )
}

/// Returns `true` if the given character is a valid `pchar`.
pub fn is_pchar(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, '%' | ':' | '@')
}

/// Returns `true` if the given character is valid in `reg-name`.
pub fn is_reg_name_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, '%')
}

/// Returns `true` if the given character is valid in `userinfo`.
pub fn is_userinfo_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, '%' | ':')
}

/// Returns `true` if the given character is valid within an `IP-literal` body.
pub fn is_ip_literal_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char) || is_sub_delim(char) || matches!(char, ':')
}

/// Returns `true` if the given slice looks like an `IPv6address` or `IPvFuture` payload.
pub fn is_ip_literal_body(bytes: &[u8]) -> bool {
    bytes.contains(&b':') || matches!(bytes.first(), Some(b'v' | b'V')) && bytes.contains(&b'.')
}

/// Parses a `uri-host`.
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
#[inline]
pub fn parse_reg_name<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    take_while(1.., is_reg_name_char).void().parse_next(input)
}

/// Parses a `port`.
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
