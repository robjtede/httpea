use core::str;

use winnow::prelude::*;

use crate::{ParseFieldNameError, parse_field_name};

/// Field name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldName<'a> {
    pub(crate) slice: &'a str,
}

impl_more::forward_display!(FieldName<'_> => slice);

impl<'a> FieldName<'a> {
    /// Parses a field name from bytes.
    pub fn try_from_slice(slice: &'a [u8]) -> Result<Self, ParseFieldNameError> {
        parse_field_name
            .parse(slice)
            .map_err(|_| ParseFieldNameError)?;

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

#[cfg(test)]
mod tests {
    use core::str;

    use quickcheck_macros::quickcheck;

    use super::FieldName;

    #[quickcheck]
    fn all_valid_field_names_are_also_utf8(bytes: Vec<u8>) -> bool {
        if FieldName::try_from_slice(&bytes).is_ok() {
            str::from_utf8(&bytes).is_ok()
        } else {
            true
        }
    }

    #[test]
    fn rejects_invalid_field_name() {
        assert_eq!(
            FieldName::try_from_slice(b"bad name"),
            Err(crate::ParseFieldNameError)
        );
    }
}
