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
///
/// # BNF
///
/// ```text
/// chunk      = chunk-size [ chunk-ext ] CRLF
///              chunk-data CRLF
/// chunk-size = 1*HEXDIG
/// chunk-data = 1*OCTET
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9112::parse_chunk;
///
/// let size = parse_chunk.parse(&b"4\r\nWiki\r\n"[..]).unwrap();
/// assert_eq!(size, 4);
/// ```
///
/// See: [RFC 9112 §7.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1)
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
///
/// # BNF
///
/// ```text
/// chunk-size = 1*HEXDIG
/// chunk-ext  = *( BWS ";" BWS chunk-ext-name
///                [ BWS "=" BWS chunk-ext-val ] )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9112::parse_chunk_header;
///
/// let (rest, size) = parse_chunk_header
///     .parse_peek(&b"000a;foo=bar\r\npayload"[..])
///     .unwrap();
/// assert_eq!(rest, b"payload");
/// assert_eq!(size, 10);
/// ```
///
/// See:
/// - [RFC 9112 §7.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1)
/// - [RFC 9112 §7.1.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1)
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
///
/// # BNF
///
/// ```text
/// chunk-size = 1*HEXDIG
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9112::parse_chunk_size;
///
/// let (rest, size) = parse_chunk_size.parse_peek(&b"000a;ext"[..]).unwrap();
/// assert_eq!(rest, b";ext");
/// assert_eq!(size, 10);
/// ```
///
/// See: [RFC 9112 §7.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1)
#[inline]
pub fn parse_chunk_size<I>(input: &mut I) -> ModalResult<usize>
where
    I: Stream<Token = u8> + StreamIsPartial,
    I::Slice: AsRef<[u8]>,
{
    parse_chunk_size_inner(input).map(|(size, _)| size)
}

/// Parses exactly `size` octets of `chunk-data`.
///
/// # BNF
///
/// ```text
/// chunk-data = 1*OCTET
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9112::parse_chunk_data;
///
/// let (rest, ()) = parse_chunk_data(4).parse_peek(&b"Wiki\r\n"[..]).unwrap();
/// assert_eq!(rest, b"\r\n");
/// ```
///
/// See: [RFC 9112 §7.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1)
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
///
/// # BNF
///
/// ```text
/// last-chunk = 1*("0") [ chunk-ext ] CRLF
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9112::parse_last_chunk;
///
/// let (rest, ()) = parse_last_chunk.parse_peek(&b"0;sig=ok\r\ntrailers"[..]).unwrap();
/// assert_eq!(rest, b"trailers");
/// ```
///
/// See: [RFC 9112 §7.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1)
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
///
/// # BNF
///
/// ```text
/// chunk-ext      = *( BWS ";" BWS chunk-ext-name
///                     [ BWS "=" BWS chunk-ext-val ] )
/// chunk-ext-name = token
/// chunk-ext-val  = token / quoted-string
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9112::parse_chunk_ext;
///
/// let (rest, ()) = parse_chunk_ext
///     .parse_peek(&b";foo=bar; baz = \"qux\"\r\n"[..])
///     .unwrap();
/// assert_eq!(rest, b"\r\n");
/// ```
///
/// See: [RFC 9112 §7.1.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1)
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
///
/// # BNF
///
/// ```text
/// ";" BWS chunk-ext-name [ BWS "=" BWS chunk-ext-val ]
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9112::parse_chunk_ext_param;
///
/// let (rest, ()) = parse_chunk_ext_param.parse_peek(&b";foo=bar rest"[..]).unwrap();
/// assert_eq!(rest, b" rest");
/// ```
///
/// See: [RFC 9112 §7.1.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1)
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
///
/// # BNF
///
/// ```text
/// chunk-ext-val = token / quoted-string
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9112::parse_chunk_ext_val;
///
/// let (rest, ()) = parse_chunk_ext_val.parse_peek(&b"\"qux\";"[..]).unwrap();
/// assert_eq!(rest, b";");
/// ```
///
/// See: [RFC 9112 §7.1.1](https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chunk_and_header_forms() {
        assert_eq!(
            parse_chunk.parse_peek(&b"4\r\nWiki\r\nrest"[..]),
            Ok((&b"rest"[..], 4))
        );
        assert_eq!(
            parse_chunk_header.parse_peek(&b"000a;foo=bar\r\npayload"[..]),
            Ok((&b"payload"[..], 10))
        );
        assert_eq!(
            parse_chunk_size.parse_peek(&b"0F;ext"[..]),
            Ok((&b";ext"[..], 15))
        );
    }

    #[test]
    fn rejects_invalid_non_terminal_chunks() {
        assert!(parse_chunk.parse_peek(&b"0\r\n\r\n"[..]).is_err());
        assert!(parse_chunk.parse_peek(&b"4\r\nabc\r\n"[..]).is_err());
        assert!(parse_chunk_header.parse_peek(&b"4\n"[..]).is_err());
        assert!(parse_chunk_size.parse_peek(&b"xyz"[..]).is_err());
    }

    #[test]
    fn parses_chunk_data_of_exact_size() {
        assert_eq!(
            parse_chunk_data(4).parse_peek(&b"Wiki\r\n"[..]),
            Ok((&b"\r\n"[..], ()))
        );
    }

    #[test]
    fn rejects_zero_length_or_truncated_chunk_data() {
        assert!(parse_chunk_data(0).parse_peek(&b"anything"[..]).is_err());
        assert!(parse_chunk_data(4).parse_peek(&b"abc"[..]).is_err());
    }

    #[test]
    fn parses_last_chunk_with_optional_extensions() {
        assert_eq!(
            parse_last_chunk.parse_peek(&b"000;sig=ok\r\ntrailers"[..]),
            Ok((&b"trailers"[..], ()))
        );
    }

    #[test]
    fn rejects_non_last_chunk_in_last_chunk_parser() {
        assert!(parse_last_chunk.parse_peek(&b"1\r\n"[..]).is_err());
    }

    #[test]
    fn parses_chunk_extensions() {
        assert_eq!(
            parse_chunk_ext.parse_peek(&b"\r\n"[..]),
            Ok((&b"\r\n"[..], ()))
        );
        assert_eq!(
            parse_chunk_ext.parse_peek(&b" ;foo=bar; baz = \"qux\"\r\n"[..]),
            Ok((&b"\r\n"[..], ()))
        );
        assert_eq!(
            parse_chunk_ext_param.parse_peek(&b";foo=bar rest"[..]),
            Ok((&b" rest"[..], ()))
        );
        assert_eq!(
            parse_chunk_ext_param.parse_peek(&b" ; token = \"quoted\"!"[..]),
            Ok((&b"!"[..], ()))
        );
        assert_eq!(
            parse_chunk_ext_val.parse_peek(&b"token;"[..]),
            Ok((&b";"[..], ()))
        );
        assert_eq!(
            parse_chunk_ext_val.parse_peek(&b"\"quoted\";"[..]),
            Ok((&b";"[..], ()))
        );
    }

    #[test]
    fn rejects_invalid_chunk_extensions() {
        assert!(parse_chunk_ext_param.parse_peek(&b"foo=bar"[..]).is_err());
        assert!(parse_chunk_ext_param.parse_peek(&b"; =bad"[..]).is_err());
        assert!(
            parse_chunk_ext_val
                .parse_peek(&b"\"unterminated"[..])
                .is_err()
        );
    }

    #[test]
    fn rejects_overflowing_chunk_size() {
        let huge = [b'F'; 100];
        assert!(parse_chunk_size.parse_peek(huge.as_slice()).is_err());
    }

    #[test]
    fn parses_internal_chunk_helpers() {
        assert_eq!(
            parse_chunk_size_inner.parse_peek(&b"000f rest"[..]),
            Ok((&b" rest"[..], (15, false)))
        );
        assert_eq!(
            parse_crlf.parse_peek(&b"\r\nbody"[..]),
            Ok((&b"body"[..], ()))
        );
        assert!(parse_crlf.parse_peek(&b"\n"[..]).is_err());

        assert!(is_hex_digit(b'F'));
        assert!(!is_hex_digit(b'g'));
        assert_eq!(hex_digit_value(b'0'), Some(0));
        assert_eq!(hex_digit_value(b'a'), Some(10));
        assert_eq!(hex_digit_value(b'F'), Some(15));
        assert_eq!(hex_digit_value(b'x'), None);

        assert!(has_chunk_ext_prefix(b" ;foo"));
        assert!(!has_chunk_ext_prefix(b" \t"));
        assert!(!has_chunk_ext_prefix(b" data"));
    }
}
