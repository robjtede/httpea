//! HTTP versions.
//!
//! See [`Version`].

/// HTTP versions.
#[derive(Debug, Clone)]
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
