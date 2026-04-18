//! HTTP status code.
//!
//! See [`StatusCode`].

/// Status code.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusCode {
    /// invariant: numeric code is always within `100..=999`
    code: u16,
}

impl StatusCode {
    /// Constructs status code from numeric representation.
    ///
    /// # Panics
    ///
    /// Panics if `code` is not within the `100..=999` range.
    pub fn from_u16(code: u16) -> Self {
        match code {
            100..=999 => Self { code },
            _ => panic!("Invalid status code: {code}"),
        }
    }

    /// Returns status code as ASCII bytes.
    pub fn as_text_bytes(&self) -> [u8; 3] {
        const DIGITS_START: u8 = b'0';

        let n = self.code;

        let d100 = (n / 100) as u8;
        let d10 = ((n / 10) % 10) as u8;
        let d1 = (n % 10) as u8;

        [DIGITS_START + d100, DIGITS_START + d10, DIGITS_START + d1]
    }
}
