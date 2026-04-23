use core::str;

use winnow::{error::ContextError, prelude::*};

use crate::parse_field_name;

/// Field name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldName<'a> {
    pub(crate) slice: &'a str,
}

impl<'a> FieldName<'a> {
    /// Parses a field name from bytes.
    pub fn try_from_slice(
        slice: &'a [u8],
    ) -> Result<Self, winnow::error::ParseError<&'a [u8], ContextError>> {
        parse_field_name.parse(slice)?;

        Ok(Self {
            // SAFETY: all valid field names are a subset of UTF-8; validated with property tests
            slice: unsafe { str::from_utf8_unchecked(slice) },
        })
    }

    /// Returns field name as a string slice.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.slice
    }

    /// Returns field name as bytes.
    #[inline]
    pub fn as_slice(&self) -> &'a [u8] {
        self.slice.as_bytes()
    }
}

impl_more::forward_display!(FieldName<'_> => slice);
