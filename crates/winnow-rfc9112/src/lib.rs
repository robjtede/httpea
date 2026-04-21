//! Winnow parsers for reusable HTTP/1.1 syntax productions from
//! [RFC 9112](https://datatracker.ietf.org/doc/html/rfc9112).

#![cfg_attr(docsrs, feature(doc_cfg))]

use winnow::{
    combinator::{alt, opt},
    error::ErrMode,
    prelude::*,
    stream::{Compare, Stream, StreamIsPartial},
};
pub use winnow_rfc9110::{parse_bws, parse_quoted_pair, parse_quoted_string, parse_token};

/// Parses a complete non-terminal chunk and returns its decoded size in octets.
#[inline]
pub fn parse_chunk<I>(input: &mut I) -> ModalResult<usize>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    let checkpoint = input.checkpoint();
    let size = parse_chunk_header.parse_next(input)?;

    if size == 0 {
        input.reset(&checkpoint);
        return Err(ErrMode::from_input(&*input));
    }

    parse_chunk_data(size).parse_next(input)?;
    parse_crlf(input)?;

    Ok(size)
}

/// Parses a `chunk-size [ chunk-ext ] CRLF` header and returns the decoded chunk size in octets.
#[inline]
pub fn parse_chunk_header<I>(input: &mut I) -> ModalResult<usize>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    let size = parse_chunk_size.parse_next(input)?;
    parse_chunk_ext.parse_next(input)?;
    parse_crlf(input)?;

    Ok(size)
}

/// Parses `chunk-size` and returns its decoded numeric value.
#[inline]
pub fn parse_chunk_size<I>(input: &mut I) -> ModalResult<usize>
where
    I: Stream<Token = u8> + StreamIsPartial,
    I::Slice: AsRef<[u8]>,
{
    parse_chunk_size_inner(input).map(|(size, _)| size)
}

/// Parses exactly `size` octets of `chunk-data`.
#[inline]
pub fn parse_chunk_data<I>(size: usize) -> impl Parser<I, (), ErrMode<winnow::error::ContextError>>
where
    I: Stream<Token = u8> + StreamIsPartial,
    I::Slice: AsRef<[u8]>,
{
    move |input: &mut I| {
        if size == 0 {
            return Err(ErrMode::from_input(&*input));
        }

        let remaining = input.peek_slice(input.eof_offset());

        if remaining.as_ref().len() < size {
            return Err(ErrMode::from_input(&*input));
        }

        let _ = input.next_slice(size);
        Ok(())
    }
}

/// Parses `last-chunk`.
#[inline]
pub fn parse_last_chunk<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    let checkpoint = input.checkpoint();
    let (_, is_last) = parse_chunk_size_inner(input)?;

    if !is_last {
        input.reset(&checkpoint);
        return Err(ErrMode::from_input(&*input));
    }

    parse_chunk_ext.parse_next(input)?;
    parse_crlf(input)
}

/// Parses `chunk-ext`.
#[inline]
pub fn parse_chunk_ext<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    while has_chunk_ext_prefix(input.peek_slice(input.eof_offset()).as_ref()) {
        parse_chunk_ext_param.parse_next(input)?;
    }

    Ok(())
}

/// Parses one `;`-prefixed chunk extension parameter.
#[inline]
pub fn parse_chunk_ext_param<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    parse_bws.parse_next(input)?;
    b';'.parse_next(input)?;
    parse_bws.parse_next(input)?;
    parse_token.parse_next(input)?;
    opt((parse_bws, b'=', parse_bws, parse_chunk_ext_val)).parse_next(input)?;

    Ok(())
}

/// Parses `chunk-ext-val`.
#[inline]
pub fn parse_chunk_ext_val<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    alt((parse_token, parse_quoted_string)).parse_next(input)
}

fn parse_chunk_size_inner<I>(input: &mut I) -> ModalResult<(usize, bool)>
where
    I: Stream<Token = u8> + StreamIsPartial,
    I::Slice: AsRef<[u8]>,
{
    let remaining = input.peek_slice(input.eof_offset());
    let remaining = remaining.as_ref();
    let mut len = 0usize;
    let mut size = 0usize;
    let mut is_last = true;

    while let Some(&byte) = remaining.get(len) {
        if !is_hex_digit(byte) {
            break;
        }

        let digit = hex_digit_value(byte).ok_or_else(|| ErrMode::from_input(&*input))?;

        size = size
            .checked_mul(16)
            .and_then(|size| size.checked_add(digit))
            .ok_or_else(|| ErrMode::from_input(&*input))?;
        is_last &= byte == b'0';
        len += 1;
    }

    if len == 0 {
        return Err(ErrMode::from_input(&*input));
    }

    let _ = input.next_slice(len);

    Ok((size, is_last))
}

fn parse_crlf<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
{
    b'\r'.parse_next(input)?;
    b'\n'.parse_next(input)?;
    Ok(())
}

fn is_hex_digit(byte: u8) -> bool {
    byte.is_ascii_hexdigit()
}

fn hex_digit_value(byte: u8) -> Option<usize> {
    Some(match byte {
        b'0'..=b'9' => (byte - b'0') as usize,
        b'a'..=b'f' => (byte - b'a' + 10) as usize,
        b'A'..=b'F' => (byte - b'A' + 10) as usize,
        _ => return None,
    })
}

fn has_chunk_ext_prefix(bytes: &[u8]) -> bool {
    matches!(
        bytes.iter().position(|&byte| !winnow_rfc9110::is_ows_byte(byte)),
        Some(index) if bytes[index] == b';'
    )
}
