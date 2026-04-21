//! Winnow parsers for reusable HTTP syntax productions from
//! [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110).

#![cfg_attr(docsrs, feature(doc_cfg))]

use winnow::{
    error::ErrMode,
    prelude::*,
    stream::{Compare, Stream, StreamIsPartial},
    token::take_while,
};

/// Parses `token`.
///
/// ```plain
/// token = 1*tchar
/// ```
#[inline]
pub fn parse_token<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    take_while(1.., is_tchar).void().parse_next(input)
}

/// Parses an HTTP field name.
#[inline]
pub fn parse_field_name<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    parse_token.parse_next(input)
}

/// Parses a trimmed HTTP field value.
#[inline]
pub fn parse_field_value<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
    I::Slice: AsRef<[u8]>,
{
    take_while(.., is_field_value_byte)
        .verify(|slice: &I::Slice| {
            let bytes = slice.as_ref();

            bytes.is_empty()
                || (bytes.first().copied().is_some_and(is_field_vchar)
                    && bytes.last().copied().is_some_and(is_field_vchar))
        })
        .void()
        .parse_next(input)
}

/// Parses optional whitespace.
#[inline]
pub fn parse_ows<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    take_while(.., is_ows_byte).void().parse_next(input)
}

/// Parses bad whitespace.
#[inline]
pub fn parse_bws<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    parse_ows.parse_next(input)
}

/// Parses `quoted-string`.
#[inline]
pub fn parse_quoted_string<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    b'"'.parse_next(input)?;

    loop {
        let remaining = input.peek_slice(input.eof_offset());
        let remaining = remaining.as_ref();

        match remaining.first().copied() {
            Some(b'"') => return b'"'.parse_next(input).map(|_| ()),
            Some(b'\\') => parse_quoted_pair.parse_next(input)?,
            Some(byte) if is_qdtext_byte(byte) => {
                let len = remaining
                    .iter()
                    .take_while(|&&byte| is_qdtext_byte(byte))
                    .count();
                let _ = input.next_slice(len);
            }
            _ => return Err(ErrMode::from_input(&*input)),
        }
    }
}

/// Parses `quoted-pair`.
#[inline]
pub fn parse_quoted_pair<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
{
    b'\\'.parse_next(input)?;
    take_while(1..=1, is_quoted_pair_byte)
        .void()
        .parse_next(input)
}

/// Returns `true` if the given byte is valid in `tchar`.
pub fn is_tchar(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
    )
}

/// Returns `true` if the given byte is valid as `field-vchar`.
pub fn is_field_vchar(byte: u8) -> bool {
    matches!(byte, 0x21..=0x7E | 0x80..=0xFF)
}

/// Returns `true` if the given byte can appear inside a trimmed field value.
pub fn is_field_value_byte(byte: u8) -> bool {
    is_field_vchar(byte) || is_ows_byte(byte)
}

/// Returns `true` if the given byte is optional whitespace.
pub fn is_ows_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

fn is_qdtext_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b' ' | 0x21 | 0x23..=0x5B | 0x5D..=0x7E | 0x80..=0xFF)
}

fn is_quoted_pair_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b' ' | 0x21..=0x7E | 0x80..=0xFF)
}
