//! HTTP fields (headers & trailers).

/// Field name.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldName<'a> {
    slice: &'a [u8],
}

impl<'a> FieldName<'a> {
    /// Constructs field name from bytes.
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self { slice }
    }

    /// Returns field name as bytes.
    pub fn as_slice(&self) -> &'a [u8] {
        self.slice
    }
}

/// Field value.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldValue<'a> {
    slice: &'a [u8],
}

impl<'a> FieldValue<'a> {
    /// Constructs field value from bytes.
    pub fn from_slice(slice: &'a [u8]) -> Self {
        Self { slice }
    }

    /// Returns field value as bytes.
    pub fn as_slice(&self) -> &'a [u8] {
        self.slice
    }
}
