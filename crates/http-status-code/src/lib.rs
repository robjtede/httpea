//! HTTP status code.
//!
//! See [`StatusCode`].

#![cfg_attr(docsrs, feature(doc_cfg))]

/// Status code.
#[derive(Debug, Clone, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructs_status_codes_within_range() {
        assert_eq!(StatusCode::from_u16(100).as_text_bytes(), *b"100");
        assert_eq!(StatusCode::from_u16(204).as_text_bytes(), *b"204");
        assert_eq!(StatusCode::from_u16(999).as_text_bytes(), *b"999");
    }

    #[test]
    #[should_panic(expected = "Invalid status code: 99")]
    fn rejects_status_codes_below_range() {
        let _ = StatusCode::from_u16(99);
    }

    #[test]
    #[should_panic(expected = "Invalid status code: 1000")]
    fn rejects_status_codes_above_range() {
        let _ = StatusCode::from_u16(1000);
    }
}
