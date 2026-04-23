//! HTTP method.

#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate alloc;

use alloc::boxed::Box;
use core::str;

/// `CONNECT` method.
pub const CONNECT: Method = Method {
    repr: Repr::WellKnown(WellKnown::Connect),
};

/// `DELETE` method.
pub const DELETE: Method = Method {
    repr: Repr::WellKnown(WellKnown::Delete),
};

/// `GET` method.
pub const GET: Method = Method {
    repr: Repr::WellKnown(WellKnown::Get),
};

/// `HEAD` method.
pub const HEAD: Method = Method {
    repr: Repr::WellKnown(WellKnown::Head),
};

/// `OPTIONS` method.
pub const OPTIONS: Method = Method {
    repr: Repr::WellKnown(WellKnown::Options),
};

/// `PATCH` method.
pub const PATCH: Method = Method {
    repr: Repr::WellKnown(WellKnown::Patch),
};

/// `POST` method.
pub const POST: Method = Method {
    repr: Repr::WellKnown(WellKnown::Post),
};

/// `PUT` method.
pub const PUT: Method = Method {
    repr: Repr::WellKnown(WellKnown::Put),
};

/// `QUERY` method.
pub const QUERY: Method = Method {
    repr: Repr::WellKnown(WellKnown::Query),
};

/// `TRACE` method.
pub const TRACE: Method = Method {
    repr: Repr::WellKnown(WellKnown::Trace),
};

/// HTTP method.
#[derive(Debug, Clone)]
pub struct Method {
    repr: Repr,
}

impl Method {
    /// Parses an HTTP method from the request-line method component.
    pub fn try_from_slice(input: &[u8]) -> Result<Self, ParseMethodError> {
        validate_method_bytes(input)?;

        // SAFETY: just validated input as HTTP `token` which is a subset of UTF-8
        let method = unsafe { str::from_utf8_unchecked(input) };

        Ok(match classify_well_known(method) {
            Some(well_known) => Self {
                repr: Repr::WellKnown(well_known),
            },
            None => Self {
                repr: Repr::HeapExtension(Box::from(method)),
            },
        })
    }

    /// Constructs an HTTP method from a static string.
    ///
    /// This constructor is usable in const contexts and validates that the input is in the
    /// expected method form: non-empty, composed only of HTTP `tchar` bytes, and without
    /// lowercase ASCII letters.
    ///
    /// # Panics
    ///
    /// Panics if `input` is empty, contains an invalid token byte, or contains lowercase ASCII.
    #[track_caller]
    pub const fn from_static(method: &'static str) -> Self {
        validate_static_method_form(method);

        match classify_well_known(method) {
            Some(well_known) => Self {
                repr: Repr::WellKnown(well_known),
            },
            None => Self {
                repr: Repr::StaticExtension(method),
            },
        }
    }

    /// Returns a string representation of the method.
    pub fn as_str(&self) -> &str {
        match &self.repr {
            Repr::WellKnown(well_known) => well_known.as_str(),
            Repr::StaticExtension(string) => string,
            Repr::HeapExtension(string) => string,
        }
    }
}

impl PartialEq for Method {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Method {}

const fn validate_method_bytes(input: &[u8]) -> Result<(), ParseMethodError> {
    if input.is_empty() {
        return Err(ParseMethodError::Empty);
    }

    let mut idx = 0;

    while idx < input.len() {
        let byte = input[idx];

        if !is_tchar(byte) {
            return Err(ParseMethodError::InvalidByte(byte));
        }

        idx += 1;
    }

    Ok(())
}

const fn validate_static_method_form(input: &'static str) {
    let bytes = input.as_bytes();

    match validate_method_bytes(bytes) {
        Ok(()) => {}
        Err(ParseMethodError::Empty) => panic!("Empty HTTP method"),
        Err(ParseMethodError::InvalidByte(_)) => panic!("Invalid HTTP method byte"),
    }

    let mut idx = 0;

    while idx < bytes.len() {
        if bytes[idx].is_ascii_lowercase() {
            panic!("HTTP methods must not contain lowercase ASCII");
        }

        idx += 1;
    }
}

const fn classify_well_known(input: &str) -> Option<WellKnown> {
    match input.as_bytes() {
        b"CONNECT" => Some(WellKnown::Connect),
        b"DELETE" => Some(WellKnown::Delete),
        b"GET" => Some(WellKnown::Get),
        b"HEAD" => Some(WellKnown::Head),
        b"OPTIONS" => Some(WellKnown::Options),
        b"PATCH" => Some(WellKnown::Patch),
        b"POST" => Some(WellKnown::Post),
        b"PUT" => Some(WellKnown::Put),
        b"QUERY" => Some(WellKnown::Query),
        b"TRACE" => Some(WellKnown::Trace),
        _ => None,
    }
}

/// Returns `true` if the given byte is valid in HTTP `tchar`.
///
/// See RFC 9110 Section 5.6.2, "Tokens":
/// <https://www.rfc-editor.org/rfc/rfc9110.html#section-5.6.2>.
const fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

/// Error returned when parsing an HTTP method fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMethodError {
    /// Input was empty.
    Empty,

    /// Input contained a byte that is not valid in an HTTP token.
    InvalidByte(u8),
}

#[derive(Debug, Clone)]
enum Repr {
    WellKnown(WellKnown),
    StaticExtension(&'static str),
    HeapExtension(Box<str>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WellKnown {
    Connect,
    Delete,
    Get,
    Head,
    Options,
    Patch,
    Post,
    Put,
    Query,
    Trace,
}

impl WellKnown {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Connect => "CONNECT",
            Self::Delete => "DELETE",
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Patch => "PATCH",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Query => "QUERY",
            Self::Trace => "TRACE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_well_known_methods() {
        assert_eq!(Method::try_from_slice(b"GET"), Ok(GET));
        assert_eq!(Method::try_from_slice(b"POST"), Ok(POST));
        assert_eq!(Method::try_from_slice(b"DELETE"), Ok(DELETE));
        assert_eq!(Method::try_from_slice(b"PATCH"), Ok(PATCH));
        assert_eq!(Method::try_from_slice(b"QUERY"), Ok(QUERY));
    }

    #[test]
    fn parses_remaining_well_known_methods() {
        assert_eq!(Method::try_from_slice(b"CONNECT"), Ok(CONNECT));
        assert_eq!(Method::try_from_slice(b"HEAD"), Ok(HEAD));
        assert_eq!(Method::try_from_slice(b"OPTIONS"), Ok(OPTIONS));
        assert_eq!(Method::try_from_slice(b"PUT"), Ok(PUT));
        assert_eq!(Method::try_from_slice(b"TRACE"), Ok(TRACE));
    }

    #[test]
    fn parses_extension_method() {
        let method = Method::try_from_slice(b"PROPFIND").unwrap();
        assert_eq!(method.as_str(), "PROPFIND");
    }

    #[test]
    fn constructs_well_known_method_from_static() {
        const METHOD: Method = Method::from_static("GET");

        assert_eq!(METHOD, GET);
        assert_eq!(METHOD.as_str(), "GET");
    }

    #[test]
    fn constructs_extension_method_from_static() {
        const METHOD: Method = Method::from_static("M-SEARCH");

        assert_eq!(METHOD.as_str(), "M-SEARCH");
        assert_eq!(METHOD, Method::try_from_slice(b"M-SEARCH").unwrap());
    }

    #[test]
    fn rejects_empty_method() {
        assert_eq!(Method::try_from_slice(b""), Err(ParseMethodError::Empty));
    }

    #[test]
    fn rejects_invalid_method_byte() {
        assert_eq!(
            Method::try_from_slice(b"GE T"),
            Err(ParseMethodError::InvalidByte(b' '))
        );
        assert_eq!(
            Method::try_from_slice(b"GET\r"),
            Err(ParseMethodError::InvalidByte(b'\r'))
        );
    }

    #[test]
    #[should_panic(expected = "Empty HTTP method")]
    fn from_static_rejects_empty_method() {
        let _ = Method::from_static("");
    }

    #[test]
    #[should_panic(expected = "Invalid HTTP method byte")]
    fn from_static_rejects_invalid_method_byte() {
        let _ = Method::from_static("GE T");
    }

    #[test]
    #[should_panic(expected = "HTTP methods must not contain lowercase ASCII")]
    fn from_static_rejects_lowercase_ascii() {
        let _ = Method::from_static("get");
    }

    #[test]
    fn renders_method_bytes() {
        let well_known = [
            (CONNECT, "CONNECT"),
            (DELETE, "DELETE"),
            (GET, "GET"),
            (HEAD, "HEAD"),
            (OPTIONS, "OPTIONS"),
            (PATCH, "PATCH"),
            (POST, "POST"),
            (PUT, "PUT"),
            (QUERY, "QUERY"),
            (TRACE, "TRACE"),
        ];

        for (method, expected) in well_known {
            assert_eq!(method.as_str(), expected);
        }
    }

    #[test]
    fn renders_method_str() {
        assert_eq!(GET.as_str(), "GET");
        assert_eq!(Method::from_static("M-SEARCH").as_str(), "M-SEARCH");
        assert_eq!(Method::try_from_slice(b"PRI").unwrap().as_str(), "PRI");
    }
}
