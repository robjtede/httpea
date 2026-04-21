//! Error types for request-target parsing.

/// Error that occurs when parsing a request target fails.
///
/// This error type is returned by [`RequestTarget::try_from_slice`] when the input does not
/// conform to any of the four request-target forms defined in [RFC 9112].
///
/// [`RequestTarget::try_from_slice`]: crate::RequestTarget::try_from_slice
/// [RFC 9112]: https://datatracker.ietf.org/doc/html/rfc9112
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParseRequestTargetError {
    /// Input was empty.
    EmptyInput,

    /// Input did not match any recognized request-target form.
    InvalidTarget,

    /// Input was incomplete (only when parsing with `Partial`).
    Incomplete,
}

impl_more::impl_display_enum! {
    ParseRequestTargetError:
    EmptyInput => "Input was empty",
    InvalidTarget => "Input did not match any recognized request-target form",
    Incomplete => "Input was incomplete",
}

impl_more::impl_leaf_error!(ParseRequestTargetError);
