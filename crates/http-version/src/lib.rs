//! HTTP versions.
//!
//! See [`Version`].

#![cfg_attr(docsrs, feature(doc_cfg))]

/// HTTP versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    /// `HTTP/0.9`.
    Http0_9,

    /// `HTTP/1.0`.
    Http1_0,

    /// `HTTP/1.1`.
    Http1_1,

    /// `HTTP/2`.
    Http2,

    /// `HTTP/3`.
    Http3,
}

impl Version {
    /// Parses an HTTP version from the request-line version component.
    pub fn try_from_slice(input: &[u8]) -> Result<Self, ParseVersionError> {
        match input {
            b"HTTP/0.9" => Ok(Self::Http0_9),
            b"HTTP/1.0" => Ok(Self::Http1_0),
            b"HTTP/1.1" => Ok(Self::Http1_1),
            b"HTTP/2" | b"HTTP/2.0" => Ok(Self::Http2),
            b"HTTP/3" | b"HTTP/3.0" => Ok(Self::Http3),
            _ => Err(ParseVersionError),
        }
    }

    /// Returns the canonical HTTP version bytes.
    pub fn as_slice(self) -> &'static [u8] {
        match self {
            Self::Http0_9 => b"HTTP/0.9",
            Self::Http1_0 => b"HTTP/1.0",
            Self::Http1_1 => b"HTTP/1.1",
            Self::Http2 => b"HTTP/2",
            Self::Http3 => b"HTTP/3",
        }
    }
}

/// Error returned when parsing an HTTP version fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseVersionError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_http_1_versions() {
        assert_eq!(Version::try_from_slice(b"HTTP/0.9"), Ok(Version::Http0_9));
        assert_eq!(Version::try_from_slice(b"HTTP/1.0"), Ok(Version::Http1_0));
        assert_eq!(Version::try_from_slice(b"HTTP/1.1"), Ok(Version::Http1_1));
    }

    #[test]
    fn parses_http_2_and_3_versions() {
        assert_eq!(Version::try_from_slice(b"HTTP/2"), Ok(Version::Http2));
        assert_eq!(Version::try_from_slice(b"HTTP/2.0"), Ok(Version::Http2));
        assert_eq!(Version::try_from_slice(b"HTTP/3"), Ok(Version::Http3));
        assert_eq!(Version::try_from_slice(b"HTTP/3.0"), Ok(Version::Http3));
    }

    #[test]
    fn rejects_invalid_versions() {
        assert_eq!(Version::try_from_slice(b""), Err(ParseVersionError));
        assert_eq!(Version::try_from_slice(b"HTTP/1"), Err(ParseVersionError));
        assert_eq!(Version::try_from_slice(b"HTP/1.1"), Err(ParseVersionError));
        assert_eq!(Version::try_from_slice(b"HTTP/9.9"), Err(ParseVersionError));
    }
}
