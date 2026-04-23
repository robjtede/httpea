use winnow::{error::ContextError, prelude::*};

use crate::parse_field_value;

/// Field value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldValue<'a> {
    pub(crate) slice: &'a [u8],
}

impl<'a> FieldValue<'a> {
    /// Parses a field value from bytes.
    pub fn try_from_slice(
        slice: &'a [u8],
    ) -> Result<Self, winnow::error::ParseError<&'a [u8], ContextError>> {
        parse_field_value.parse(slice)?;

        Ok(Self { slice })
    }

    /// Returns field value as bytes.
    #[inline]
    pub fn as_slice(&self) -> &'a [u8] {
        self.slice
    }
}
