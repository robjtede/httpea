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
/// # BNF
///
/// ```text
/// token = 1*tchar
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9110::parse_token;
///
/// let (rest, ()) = parse_token.parse_peek(&b"gzip,"[..]).unwrap();
/// assert_eq!(rest, b",");
/// ```
///
/// See: [RFC 9110 §5.6.2](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.2)
#[inline]
pub fn parse_token<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    take_while(1.., is_tchar).void().parse_next(input)
}

/// Parses an HTTP field name.
///
/// # BNF
///
/// ```text
/// field-name = token
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9110::parse_field_name;
///
/// let (rest, ()) = parse_field_name.parse_peek(&b"content-type: text/plain"[..]).unwrap();
/// assert_eq!(rest, b": text/plain");
/// ```
///
/// See: [RFC 9110 §5.1](https://datatracker.ietf.org/doc/html/rfc9110#section-5.1)
#[inline]
pub fn parse_field_name<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    parse_token.parse_next(input)
}

/// Parses a trimmed HTTP field value.
///
/// # BNF
///
/// ```text
/// field-value   = *field-content
/// field-content = field-vchar
///                 [ 1*( SP / HTAB / field-vchar ) field-vchar ]
/// field-vchar   = VCHAR / obs-text
/// obs-text      = %x80-FF
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9110::parse_field_value;
///
/// let (rest, ()) = parse_field_value
///     .parse_peek(&b"text/plain; charset=utf-8\r\n"[..])
///     .unwrap();
/// assert_eq!(rest, b"\r\n");
/// ```
///
/// See: [RFC 9110 §5.5](https://datatracker.ietf.org/doc/html/rfc9110#section-5.5)
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
///
/// # BNF
///
/// ```text
/// OWS = *( SP / HTAB )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9110::parse_ows;
///
/// let (rest, ()) = parse_ows.parse_peek(&b" \tvalue"[..]).unwrap();
/// assert_eq!(rest, b"value");
/// ```
///
/// See: [RFC 9110 §5.6.3](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.3)
#[inline]
pub fn parse_ows<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    take_while(.., is_ows_byte).void().parse_next(input)
}

/// Parses bad whitespace.
///
/// # BNF
///
/// ```text
/// BWS = OWS
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9110::parse_bws;
///
/// let (rest, ()) = parse_bws.parse_peek(&b"\t ;foo"[..]).unwrap();
/// assert_eq!(rest, b";foo");
/// ```
///
/// See: [RFC 9110 §5.6.3](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.3)
#[inline]
pub fn parse_bws<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    parse_ows.parse_next(input)
}

/// Parses `quoted-string`.
///
/// # BNF
///
/// ```text
/// quoted-string = DQUOTE *( qdtext / quoted-pair ) DQUOTE
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9110::parse_quoted_string;
///
/// let (rest, ()) = parse_quoted_string
///     .parse_peek(&b"\"sig\\\\value\";"[..])
///     .unwrap();
/// assert_eq!(rest, b";");
/// ```
///
/// See: [RFC 9110 §5.6.4](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4)
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
///
/// # BNF
///
/// ```text
/// quoted-pair = "\" ( HTAB / SP / VCHAR / obs-text )
/// ```
///
/// # Examples
///
/// ```
/// use winnow::Parser as _;
/// use winnow_rfc9110::parse_quoted_pair;
///
/// let (rest, ()) = parse_quoted_pair.parse_peek(&br#"\"rest"#[..]).unwrap();
/// assert_eq!(rest, b"rest");
/// ```
///
/// See: [RFC 9110 §5.6.4](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4)
#[inline]
pub fn parse_quoted_pair<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
{
    (b'\\', take_while(1..=1, is_quoted_pair_byte))
        .void()
        .parse_next(input)
}

/// Returns `true` if the given byte is valid in `tchar`.
#[inline]
pub fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
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
        )
}

/// Returns `true` if the given byte is valid as `field-vchar`.
#[inline]
pub fn is_field_vchar(byte: u8) -> bool {
    matches!(byte, 0x21..=0x7E | 0x80..=0xFF)
}

/// Returns `true` if the given byte can appear inside a trimmed field value.
#[inline]
pub fn is_field_value_byte(byte: u8) -> bool {
    is_field_vchar(byte) || is_ows_byte(byte)
}

/// Returns `true` if the given byte is optional whitespace.
#[inline]
pub fn is_ows_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// Returns `true` if the given byte is valid as `qdtext`.
///
/// This matches the octets allowed unescaped inside a `quoted-string`, excluding
/// DQUOTE (`"`) and backslash (`\`), which are handled separately by the parser.
///
/// See: [RFC 9110 §5.6.4](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4)
fn is_qdtext_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b' ' | 0x21 | 0x23..=0x5B | 0x5D..=0x7E | 0x80..=0xFF)
}

/// Returns `true` if the given byte is valid after the leading backslash in a `quoted-pair`.
///
/// This matches the `quoted-pair` production body: HTAB, SP, VCHAR, or `obs-text`.
///
/// See: [RFC 9110 §5.6.4](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4)
fn is_quoted_pair_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b' ' | 0x21..=0x7E | 0x80..=0xFF)
}
