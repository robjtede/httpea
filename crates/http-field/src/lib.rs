//! HTTP fields (headers & trailers).
//!
//! Parsing follows the generic field syntax from
//! [RFC 9112](https://datatracker.ietf.org/doc/html/rfc9112#section-5) and the field-name /
//! field-value rules from [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110#section-5.1),
//! [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110#section-5.5), and
//! [RFC 9110](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.2).

#![cfg_attr(docsrs, feature(doc_cfg))]

use core::ops::Range;

use winnow::{
    error::{ContextError, ErrMode},
    prelude::*,
    stream::LocatingSlice,
};

mod parsing;

/// Entire HTTP field line, excluding any trailing CRLF.
///
/// ```plain
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

/// Field name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldName<'a> {
    slice: &'a [u8],
}

/// Field value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldValue<'a> {
    slice: &'a [u8],
}

impl<'a> Field<'a> {
    /// Parses an entire HTTP field line from bytes, excluding the trailing CRLF.
    pub fn try_from_slice(input: &'a [u8]) -> Result<Self, ErrMode<ContextError>> {
        let mut located = LocatingSlice::new(input);
        let indices = parsing::parse_field_indices.parse_next(&mut located)?;

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
            slice: slice_range(self.inner, &self.name),
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

impl<'a> FieldName<'a> {
    /// Constructs field name from bytes.
    #[inline]
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self { slice }
    }

    /// Parses a field name from bytes.
    pub fn try_from_slice(
        slice: &'a [u8],
    ) -> Result<Self, winnow::error::ParseError<&'a [u8], ContextError>> {
        parsing::parse_field_name.parse(slice)?;

        Ok(Self { slice })
    }

    /// Returns field name as bytes.
    #[inline]
    pub fn as_slice(&self) -> &'a [u8] {
        self.slice
    }
}

impl<'a> FieldValue<'a> {
    /// Constructs field value from bytes.
    #[inline]
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self { slice }
    }

    /// Parses a field value from bytes.
    pub fn try_from_slice(
        slice: &'a [u8],
    ) -> Result<Self, winnow::error::ParseError<&'a [u8], ContextError>> {
        parsing::parse_field_value.parse(slice)?;

        Ok(Self { slice })
    }

    /// Returns field value as bytes.
    #[inline]
    pub fn as_slice(&self) -> &'a [u8] {
        self.slice
    }
}

#[inline]
fn slice_range<'a>(bytes: &'a [u8], range: &Range<usize>) -> &'a [u8] {
    &bytes[range.start..range.end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_field_line() {
        let field = Field::try_from_slice(b"content-type: text/plain; charset=utf-8").unwrap();

        assert_eq!(field.as_bytes(), b"content-type: text/plain; charset=utf-8");
        assert_eq!(field.name_indices(), 0..12);
        assert_eq!(field.name().as_slice(), b"content-type");
        assert_eq!(field.value_indices(), 14..39);
        assert_eq!(field.value().as_slice(), b"text/plain; charset=utf-8");
    }

    #[test]
    fn trims_optional_whitespace_around_value() {
        let field = Field::try_from_slice(b"accept:\t application/json \t").unwrap();

        assert_eq!(field.name().as_slice(), b"accept");
        assert_eq!(field.value().as_slice(), b"application/json");
    }

    #[test]
    fn allows_empty_field_value() {
        let field = Field::try_from_slice(b"x-empty:\t ").unwrap();

        assert_eq!(field.value().as_slice(), b"");
        assert_eq!(field.value_indices(), 10..10);
    }

    #[test]
    fn validates_name_and_value_components() {
        assert!(FieldName::try_from_slice(b"content-type").is_ok());
        assert!(FieldName::try_from_slice(b"content type").is_err());

        assert!(FieldValue::try_from_slice(b"text/plain; charset=utf-8").is_ok());
        assert!(FieldValue::try_from_slice(b"text/plain\r").is_err());
    }
}
