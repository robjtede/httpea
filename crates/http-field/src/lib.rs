//! HTTP fields (headers & trailers).
//!
//! Parsing follows the generic field syntax from
//! [RFC 9112 §5](https://datatracker.ietf.org/doc/html/rfc9112#section-5) and the field-name /
//! field-value rules from [RFC 9110 §5.1](https://datatracker.ietf.org/doc/html/rfc9110#section-5.1),
//! [RFC 9110 §5.5](https://datatracker.ietf.org/doc/html/rfc9110#section-5.5), and
//! [RFC 9110 §5.6.2](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.2).

#![cfg_attr(docsrs, feature(doc_cfg))]

mod name;
mod parsing;
mod value;

use core::ops::Range;

use winnow::error::{ContextError, ErrMode};
pub use winnow_rfc9110::{parse_field_name, parse_field_value, parse_ows};

pub use name::FieldName;
pub use value::FieldValue;

/// Entire HTTP field line, excluding any trailing CRLF.
///
/// # BNF
///
/// ```text
/// field-line = field-name ":" OWS field-value OWS
/// ```
///
/// The parsed `field-value` excludes surrounding optional whitespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field<'a> {
    inner: &'a [u8],
    name: Range<usize>,
    value: Range<usize>,
}

impl<'a> Field<'a> {
    /// Parses an entire HTTP field line from bytes, excluding the trailing CRLF.
    #[inline]
    pub fn try_from_slice(input: &'a [u8]) -> Result<Self, ErrMode<ContextError>> {
        let indices = parsing::parse_field(input)?;

        Ok(Self {
            inner: input,
            name: indices.name,
            value: indices.value,
        })
    }

    /// Returns the full field-line bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner
    }

    /// Returns the byte indices of the field name.
    #[inline]
    pub fn name_indices(&self) -> Range<usize> {
        self.name.clone()
    }

    /// Returns the parsed field name.
    #[inline]
    pub fn name(&self) -> FieldName<'a> {
        FieldName {
            // SAFETY: all valid field names are a subset of UTF-8; validated with property tests
            slice: unsafe { str::from_utf8_unchecked(slice_range(self.inner, &self.name)) },
        }
    }

    /// Returns the byte indices of the trimmed field value.
    #[inline]
    pub fn value_indices(&self) -> Range<usize> {
        self.value.clone()
    }

    /// Returns the parsed field value.
    #[inline]
    pub fn value(&self) -> FieldValue<'a> {
        FieldValue {
            slice: slice_range(self.inner, &self.value),
        }
    }
}

#[inline]
fn slice_range<'a>(bytes: &'a [u8], range: &Range<usize>) -> &'a [u8] {
    &bytes[range.start..range.end]
}

#[cfg(test)]
mod tests {
    use std::str;

    use quickcheck_macros::quickcheck;

    use super::*;

    #[test]
    fn parses_field_line() {
        let field = Field::try_from_slice(b"content-type: text/plain; charset=utf-8").unwrap();

        assert_eq!(field.as_bytes(), b"content-type: text/plain; charset=utf-8");
        assert_eq!(field.name_indices(), 0..12);
        assert_eq!(field.name().as_str(), "content-type");
        assert_eq!(field.name().as_slice(), b"content-type");
        assert_eq!(field.name().to_string(), "content-type");
        assert_eq!(field.value_indices(), 14..39);
        assert_eq!(field.value().as_slice(), b"text/plain; charset=utf-8");
    }

    #[test]
    fn trims_optional_whitespace_around_value() {
        let field = Field::try_from_slice(b"accept:\t application/json \t").unwrap();

        assert_eq!(field.name().as_str(), "accept");
        assert_eq!(field.name().as_slice(), b"accept");
        assert_eq!(field.value().as_slice(), b"application/json");
    }

    #[test]
    fn parses_empty_field_value() {
        let field = Field::try_from_slice(b"x-empty:").unwrap();

        assert_eq!(field.name().as_str(), "x-empty");
        assert_eq!(field.name().as_slice(), b"x-empty");
        assert_eq!(field.value().as_slice(), b"");
    }

    #[test]
    fn parses_empty_field_value_with_trailing_whitespace() {
        let field = Field::try_from_slice(b"x-empty:\t ").unwrap();

        assert_eq!(field.name().as_str(), "x-empty");
        assert_eq!(field.name().as_slice(), b"x-empty");
        assert_eq!(field.value().as_slice(), b"");
    }

    #[test]
    fn validates_name_and_value_components() {
        assert!(FieldName::try_from_slice(b"content-type").is_ok());
        assert!(FieldName::try_from_slice(b"content type").is_err());
        assert_eq!(
            FieldName::try_from_slice(b"content-type").unwrap().as_str(),
            "content-type"
        );

        assert!(FieldValue::try_from_slice(b"text/plain; charset=utf-8").is_ok());
        assert!(FieldValue::try_from_slice(b"text/plain\r").is_err());
    }

    #[test]
    fn rejects_truncated_field_lines() {
        assert!(Field::try_from_slice(b"content-type").is_err());
    }

    #[quickcheck]
    fn all_valid_field_names_are_also_utf8(bytes: Vec<u8>) -> bool {
        if FieldName::try_from_slice(&bytes).is_ok() {
            str::from_utf8(&bytes).is_ok()
        } else {
            true
        }
    }
}
