use core::ops::Range;

use winnow::{
    error::{ContextError, ErrMode},
    prelude::*,
    stream::{LocatingSlice, Location, Stream},
    token::literal,
};
use winnow_rfc9110::{is_ows_byte, parse_field_name, parse_field_value, parse_ows};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldIndices {
    pub(crate) name: Range<usize>,
    pub(crate) value: Range<usize>,
}

/// Parses an entire HTTP field line, excluding the trailing CRLF.
///
/// # BNF
///
/// ```text
/// field-line = field-name ":" OWS field-value OWS
/// ```
///
/// See:
/// - [RFC 9112 §5](https://datatracker.ietf.org/doc/html/rfc9112#section-5)
/// - [RFC 9110 §5.1](https://datatracker.ietf.org/doc/html/rfc9110#section-5.1)
/// - [RFC 9110 §5.5](https://datatracker.ietf.org/doc/html/rfc9110#section-5.5)
pub(crate) fn parse_field(input: &[u8]) -> Result<FieldIndices, ErrMode<ContextError>> {
    let mut input = LocatingSlice::new(input);

    let (name, _, _) =
        (parse_field_name.span(), literal(b':'), parse_ows).parse_next(&mut input)?;

    let value_start = input.current_token_start();
    let remaining = input.peek_slice(input.eof_offset());
    let trimmed_end = remaining
        .iter()
        .rposition(|&byte| !is_ows_byte(byte))
        .map_or(0, |index| index + 1);

    let mut value_input = &remaining[..trimmed_end];
    parse_field_value.parse_next(&mut value_input)?;

    if !value_input.is_empty() {
        return Err(ErrMode::from_input(&value_input));
    }

    let mut trailing_input = &remaining[trimmed_end..];
    parse_ows.parse_next(&mut trailing_input)?;

    if !trailing_input.is_empty() {
        return Err(ErrMode::from_input(&trailing_input));
    }

    Ok(FieldIndices {
        name,
        value: value_start..(value_start + trimmed_end),
    })
}

#[cfg(test)]
mod tests {
    use winnow::BStr;
    use winnow_rfc9110::{is_field_value_byte, is_field_vchar, is_tchar};

    use super::*;

    macro_rules! assert_ok_remaining {
        ($parser:expr, $input:expr, $remaining:expr $(,)?) => {
            assert_eq!(
                $parser.parse_peek(BStr::new($input)),
                Ok((BStr::new($remaining), ())),
            );
        };
    }

    #[test]
    fn validates_char_groups() {
        assert!(is_tchar(b'A'));
        assert!(is_tchar(b'~'));
        assert!(!is_tchar(b':'));
        assert!(!is_tchar(b' '));

        assert!(is_field_vchar(b'!'));
        assert!(is_field_vchar(0x80));
        assert!(!is_field_vchar(b' '));
        assert!(!is_field_vchar(b'\t'));

        assert!(is_field_value_byte(b' '));
        assert!(is_field_value_byte(b'\t'));
        assert!(is_field_value_byte(b'x'));
        assert!(!is_field_value_byte(b'\r'));
        assert!(!is_field_value_byte(0x00));
    }

    #[test]
    fn parses_field_name() {
        assert_ok_remaining!(parse_field_name, b"content-type", b"");
        assert_ok_remaining!(parse_field_name, b"etag: abc", b": abc");

        assert!(parse_field_name.parse(&b""[..]).is_err());
        assert!(parse_field_name.parse(&b"bad name"[..]).is_err());
    }

    #[test]
    fn parses_field_value() {
        assert_ok_remaining!(parse_field_value, b"", b"");
        assert_ok_remaining!(parse_field_value, b"text/plain", b"");
        assert_ok_remaining!(parse_field_value, b"text/plain\tcharset", b"");

        assert!(parse_field_value.parse(&b"text/plain\r"[..]).is_err());
        assert!(parse_field_value.parse(&b"\ttext/plain"[..]).is_err());
        assert!(parse_field_value.parse(&b"text/plain "[..]).is_err());
    }

    #[test]
    fn parses_ows() {
        assert_ok_remaining!(parse_ows, b"", b"");
        assert_ok_remaining!(parse_ows, b" \tvalue", b"value");
    }

    #[test]
    fn parses_field_line() {
        let indices = parse_field(b"content-type: text/plain").unwrap();
        assert_eq!(indices.name, 0..12);
        assert_eq!(indices.value, 14..24);
    }

    #[test]
    fn parses_empty_field_value() {
        let indices = parse_field(b"x-empty:").unwrap();

        assert_eq!(indices.name, 0..7);
        assert_eq!(indices.value, 8..8);
    }

    #[test]
    fn parses_empty_field_value_with_trailing_whitespace() {
        let indices = parse_field(b"x-empty: \t").unwrap();

        assert_eq!(indices.name, 0..7);
        assert_eq!(indices.value, 10..10);
    }

    #[test]
    fn rejects_invalid_field_lines() {
        assert!(parse_field(b"bad name: value").is_err());
        assert!(parse_field(b"name : value").is_err());
        assert!(parse_field(b"name:value\r").is_err());
        assert!(parse_field(b"name:\0").is_err());
    }

    #[test]
    fn rejects_truncated_field_lines() {
        assert!(parse_field(b"content-type").is_err());
        let indices = parse_field(b"content-type: value").unwrap();
        assert_eq!(indices.name, 0..12);
        assert_eq!(indices.value, 14..19);
    }
}
