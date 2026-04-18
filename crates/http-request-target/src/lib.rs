//! HTTP/1.1 request-target (RFC 9112) parser.

// #![no_std]
use core::ops::Range;

use winnow::{error::ContextError, prelude::*, stream::LocatingSlice};

mod error;
mod parsing;

// use crate::error::ParseRequestTargetError;

/// See <https://datatracker.ietf.org/doc/html/rfc9112#name-request-target>.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestTarget<'a> {
    /// Origin form.
    Origin(RequestTargetOrigin<'a>),

    /// ```plain
    /// absolute-form = absolute-URI
    /// GET http://www.example.org/pub/WWW/TheProject.html HTTP/1.1
    /// ```
    Absolute(RequestTargetAbsolute<'a>),

    /// ```plain
    /// authority-form = uri-host ":" port
    /// CONNECT www.example.com:80 HTTP/1.1
    /// ```
    Authority(RequestTargetAuthority<'a>),

    /// Asterisk form.
    ///
    /// ```plain
    /// asterisk-form = "*"
    /// OPTIONS * HTTP/1.1
    /// ```
    Asterisk,
}

/// Origin-form request-target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestTargetOrigin<'a> {
    inner: &'a [u8],
    path: Range<usize>,
    search: Option<Range<usize>>,
}

/// Absolute-form request-target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestTargetAbsolute<'a> {
    inner: &'a [u8],
    scheme: Range<usize>,
    authority: Range<usize>,
    userinfo: Option<Range<usize>>,
    host: Range<usize>,
    port: Option<Range<usize>>,
    path: Range<usize>,
    search: Option<Range<usize>>,
}

/// Authority-form request-target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestTargetAuthority<'a> {
    inner: &'a [u8],
    host: Range<usize>,
    port: Range<usize>,
}

impl<'a> RequestTarget<'a> {
    /// Parse request target from slice.
    pub fn try_from_slice(
        input: &'a [u8],
    ) -> Result<Self, winnow::error::ParseError<&'a [u8], ContextError>> {
        match parsing::parse_request_target_indices.parse(LocatingSlice::new(input)) {
            Ok(parsing::RequestTargetIndices::Origin(indices)) => {
                Ok(RequestTarget::Origin(RequestTargetOrigin {
                    inner: input,
                    path: indices.path,
                    search: indices.search,
                }))
            }
            Ok(parsing::RequestTargetIndices::Absolute(indices)) => {
                Ok(RequestTarget::Absolute(RequestTargetAbsolute {
                    inner: input,
                    scheme: indices.scheme,
                    authority: indices.authority,
                    userinfo: indices.userinfo,
                    host: indices.host,
                    port: indices.port,
                    path: indices.path,
                    search: indices.search,
                }))
            }
            Ok(parsing::RequestTargetIndices::Authority(indices)) => {
                Ok(RequestTarget::Authority(RequestTargetAuthority {
                    inner: input,
                    host: indices.host,
                    port: indices.port,
                }))
            }
            Ok(parsing::RequestTargetIndices::Asterisk) => Ok(RequestTarget::Asterisk),
            Err(_) => parsing::parse_request_target
                .parse(input)
                .map(|()| unreachable!("indexed parser and validator diverged")),
        }
    }
}

impl<'a> RequestTargetOrigin<'a> {
    /// Returns the full request-target bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner
    }

    /// Returns the byte indices of the `path` component.
    #[inline]
    pub fn path_indices(&self) -> Range<usize> {
        self.path.clone()
    }

    /// Returns the `path` component.
    #[inline]
    pub fn path(&self) -> &'a [u8] {
        slice_range(self.inner, &self.path)
    }

    /// Returns the byte indices of the `query` component, excluding the leading `?`.
    ///
    /// Use [`Self::search_indices`] for the same component including the `?`.
    #[inline]
    pub fn query_indices(&self) -> Option<Range<usize>> {
        self.search.as_ref().map(query_range)
    }

    /// Returns the `query` component, excluding the leading `?`.
    ///
    /// Use [`Self::search`] for the same slice including the `?`.
    #[inline]
    pub fn query(&self) -> Option<&'a [u8]> {
        optional_slice_range(self.inner, self.search.as_ref().map(query_range).as_ref())
    }

    /// Returns the byte indices of the `search` component, including the leading `?`.
    ///
    /// This is the same logical component as [`Self::query_indices`], but with the `?` included.
    #[inline]
    pub fn search_indices(&self) -> Option<Range<usize>> {
        self.search.clone()
    }

    /// Returns the `search` component, including the leading `?`.
    ///
    /// This is the same logical component as [`Self::query`], but with the `?` included.
    #[inline]
    pub fn search(&self) -> Option<&'a [u8]> {
        optional_slice_range(self.inner, self.search.as_ref())
    }
}

impl<'a> RequestTargetAbsolute<'a> {
    /// Returns the full request-target bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner
    }

    /// Returns the byte indices of the `scheme` component.
    #[inline]
    pub fn scheme_indices(&self) -> Range<usize> {
        self.scheme.clone()
    }

    /// Returns the `scheme` component.
    #[inline]
    pub fn scheme(&self) -> &'a [u8] {
        slice_range(self.inner, &self.scheme)
    }

    /// Returns the byte indices of the `authority` component.
    #[inline]
    pub fn authority_indices(&self) -> Range<usize> {
        self.authority.clone()
    }

    /// Returns the `authority` component.
    #[inline]
    pub fn authority(&self) -> &'a [u8] {
        slice_range(self.inner, &self.authority)
    }

    /// Returns the byte indices of the `userinfo` component, excluding the trailing `@`.
    #[inline]
    pub fn userinfo_indices(&self) -> Option<Range<usize>> {
        self.userinfo.clone()
    }

    /// Returns the `userinfo` component, excluding the trailing `@`.
    #[inline]
    pub fn userinfo(&self) -> Option<&'a [u8]> {
        optional_slice_range(self.inner, self.userinfo.as_ref())
    }

    /// Returns the byte indices of the `host` component.
    #[inline]
    pub fn host_indices(&self) -> Range<usize> {
        self.host.clone()
    }

    /// Returns the `host` component.
    #[inline]
    pub fn host(&self) -> &'a [u8] {
        slice_range(self.inner, &self.host)
    }

    /// Returns the byte indices of the `port` component, excluding the leading `:`.
    #[inline]
    pub fn port_indices(&self) -> Option<Range<usize>> {
        self.port.clone()
    }

    /// Returns the `port` component, excluding the leading `:`.
    #[inline]
    pub fn port(&self) -> Option<&'a [u8]> {
        optional_slice_range(self.inner, self.port.as_ref())
    }

    /// Returns the byte indices of the `path` component.
    #[inline]
    pub fn path_indices(&self) -> Range<usize> {
        self.path.clone()
    }

    /// Returns the `path` component.
    #[inline]
    pub fn path(&self) -> &'a [u8] {
        slice_range(self.inner, &self.path)
    }

    /// Returns the byte indices of the `query` component, excluding the leading `?`.
    ///
    /// Use [`Self::search_indices`] for the same component including the `?`.
    #[inline]
    pub fn query_indices(&self) -> Option<Range<usize>> {
        self.search.as_ref().map(query_range)
    }

    /// Returns the `query` component, excluding the leading `?`.
    ///
    /// Use [`Self::search`] for the same slice including the `?`.
    #[inline]
    pub fn query(&self) -> Option<&'a [u8]> {
        optional_slice_range(self.inner, self.search.as_ref().map(query_range).as_ref())
    }

    /// Returns the byte indices of the `search` component, including the leading `?`.
    ///
    /// This is the same logical component as [`Self::query_indices`], but with the `?` included.
    #[inline]
    pub fn search_indices(&self) -> Option<Range<usize>> {
        self.search.clone()
    }

    /// Returns the `search` component, including the leading `?`.
    ///
    /// This is the same logical component as [`Self::query`], but with the `?` included.
    #[inline]
    pub fn search(&self) -> Option<&'a [u8]> {
        optional_slice_range(self.inner, self.search.as_ref())
    }
}

impl<'a> RequestTargetAuthority<'a> {
    /// Returns the full request-target bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner
    }

    /// Returns the byte indices of the `host` component.
    #[inline]
    pub fn host_indices(&self) -> Range<usize> {
        self.host.clone()
    }

    /// Returns the `host` component.
    #[inline]
    pub fn host(&self) -> &'a [u8] {
        slice_range(self.inner, &self.host)
    }

    /// Returns the byte indices of the `port` component, excluding the leading `:`.
    #[inline]
    pub fn port_indices(&self) -> Range<usize> {
        self.port.clone()
    }

    /// Returns the `port` component, excluding the leading `:`.
    #[inline]
    pub fn port(&self) -> &'a [u8] {
        slice_range(self.inner, &self.port)
    }
}

#[inline]
fn slice_range<'a>(input: &'a [u8], range: &Range<usize>) -> &'a [u8] {
    &input[range.start..range.end]
}

#[inline]
fn optional_slice_range<'a>(input: &'a [u8], range: Option<&Range<usize>>) -> Option<&'a [u8]> {
    range.map(|range| slice_range(input, range))
}

#[inline]
fn query_range(search: &Range<usize>) -> Range<usize> {
    (search.start + 1)..search.end
}
