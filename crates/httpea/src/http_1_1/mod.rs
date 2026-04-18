//! HTTP/1.1 message-line parsing helpers.

use core::ops::Range;

use http_method::Method;
use http_request_target::RequestTarget;
use http_version::Version;
use winnow::{
    error::{ContextError, ErrMode},
    prelude::*,
    stream::LocatingSlice,
};

mod parsing;

/// Parsed HTTP/1.1 request line.
///
/// This is a zero-copy view over the request-line bytes, with parsed access to
/// the `method`, `request-target`, and `HTTP-version` components.
///
/// # Request Line Examples
///
/// ```plain
/// GET /where?q=now HTTP/1.1
/// HEAD / HTTP/1.0
/// ```
///
/// # BNF
///
/// ```plain
/// request-line = method SP request-target SP HTTP-version
/// ```
///
/// The input to [`Self::try_from_slice`] must be just the request line bytes,
/// without the trailing CRLF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLine<'a> {
    inner: &'a [u8],
    method: Method,
    method_range: Range<usize>,
    target: RequestTarget<'a>,
    target_range: Range<usize>,
    version: Version,
    version_range: Range<usize>,
}

impl<'a> RequestLine<'a> {
    /// Parses a request line from raw HTTP/1.1 bytes.
    pub fn try_from_slice(input: &'a [u8]) -> Result<Self, ErrMode<ContextError>> {
        let mut located = LocatingSlice::new(input);
        let indices = parsing::parse_request_line_indices.parse_next(&mut located)?;
        let method = Method::try_from_slice(slice_range(input, &indices.method))
            .map_err(|_| unreachable!("request-line parser and method parser diverged"))?;
        let target = RequestTarget::try_from_slice(slice_range(input, &indices.target))
            .map_err(|_| unreachable!("request-line parser and request-target parser diverged"))?;
        let version = Version::try_from_slice(slice_range(input, &indices.version))
            .map_err(|_| unreachable!("request-line parser and version parser diverged"))?;

        Ok(Self {
            inner: input,
            method,
            method_range: indices.method,
            target,
            target_range: indices.target,
            version,
            version_range: indices.version,
        })
    }

    /// Returns the full request-line bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner
    }

    /// Returns the byte indices of the method component.
    #[inline]
    pub fn method_indices(&self) -> Range<usize> {
        self.method_range.clone()
    }

    /// Returns the parsed request method.
    #[inline]
    pub fn method(&self) -> Method {
        self.method.clone()
    }

    /// Returns the byte indices of the request-target component.
    #[inline]
    pub fn target_indices(&self) -> Range<usize> {
        self.target_range.clone()
    }

    /// Returns the parsed request-target.
    #[inline]
    pub fn target(&self) -> &RequestTarget<'a> {
        &self.target
    }

    /// Returns the byte indices of the HTTP-version component.
    #[inline]
    pub fn version_indices(&self) -> Range<usize> {
        self.version_range.clone()
    }

    /// Returns the parsed HTTP version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }
}

#[inline]
fn slice_range<'a>(input: &'a [u8], range: &Range<usize>) -> &'a [u8] {
    &input[range.start..range.end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_request_line() {
        let line = RequestLine::try_from_slice(b"GET /where?q=now HTTP/1.1").unwrap();

        assert_eq!(line.as_bytes(), b"GET /where?q=now HTTP/1.1");
        assert_eq!(line.method_indices(), 0..3);
        assert_eq!(line.method().as_slice(), b"GET");
        assert_eq!(line.target_indices(), 4..16);
        assert_eq!(line.version_indices(), 17..25);
        assert_eq!(line.version(), Version::Http1_1);

        match line.target() {
            RequestTarget::Origin(target) => {
                assert_eq!(target.path(), b"/where");
                assert_eq!(target.query(), Some(&b"q=now"[..]));
            }
            other => panic!("expected origin-form target, got {:?}", other),
        }
    }

    #[test]
    fn parses_connect_request_line() {
        let line = RequestLine::try_from_slice(b"CONNECT www.example.com:443 HTTP/1.1").unwrap();

        assert_eq!(line.method().as_slice(), b"CONNECT");
        assert_eq!(line.version(), Version::Http1_1);

        match line.target() {
            RequestTarget::Authority(target) => {
                assert_eq!(target.host(), b"www.example.com");
                assert_eq!(target.port(), b"443");
            }
            other => panic!("expected authority-form target, got {:?}", other),
        }
    }

    #[test]
    fn rejects_invalid_request_lines() {
        assert!(RequestLine::try_from_slice(b"GET  / HTTP/1.1").is_err());
        assert!(RequestLine::try_from_slice(b"GET /\r HTTP/1.1").is_err());
        assert!(RequestLine::try_from_slice(b"GET / HTTP/2").is_err());
        assert!(RequestLine::try_from_slice(b"GET / HTTP/1.1\r\n").is_err());
    }
}
