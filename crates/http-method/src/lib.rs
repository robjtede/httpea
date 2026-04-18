//! HTTP method.

use std::boxed::Box;

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

/// `TRACE` method.
pub const TRACE: Method = Method {
    repr: Repr::WellKnown(WellKnown::Trace),
};

/// HTTP method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Method {
    repr: Repr,
}

impl Method {
    /// Parses an HTTP method from the request-line method component.
    pub fn try_from_slice(input: &[u8]) -> Result<Self, ParseMethodError> {
        if input.is_empty() {
            return Err(ParseMethodError::Empty);
        }

        if let Some(&byte) = input.iter().find(|&&byte| !is_tchar(byte)) {
            return Err(ParseMethodError::InvalidByte(byte));
        }

        Ok(match input {
            b"CONNECT" => CONNECT,
            b"DELETE" => DELETE,
            b"GET" => GET,
            b"HEAD" => HEAD,
            b"OPTIONS" => OPTIONS,
            b"PATCH" => PATCH,
            b"POST" => POST,
            b"PUT" => PUT,
            b"TRACE" => TRACE,
            _ => Self {
                repr: Repr::Extension(Box::from(input)),
            },
        })
    }

    /// Returns the method as bytes.
    pub fn as_slice(&self) -> &[u8] {
        match &self.repr {
            Repr::WellKnown(well_known) => well_known.as_slice(),
            Repr::Extension(bytes) => bytes,
        }
    }
}

/// Error returned when parsing an HTTP method fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMethodError {
    /// Input was empty.
    Empty,

    /// Input contained a byte that is not valid in an HTTP token.
    InvalidByte(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Repr {
    WellKnown(WellKnown),
    Extension(Box<[u8]>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WellKnown {
    Connect,
    Delete,
    Get,
    Head,
    Options,
    Patch,
    Post,
    Put,
    Trace,
}

impl WellKnown {
    fn as_slice(self) -> &'static [u8] {
        match self {
            Self::Connect => b"CONNECT",
            Self::Delete => b"DELETE",
            Self::Get => b"GET",
            Self::Head => b"HEAD",
            Self::Options => b"OPTIONS",
            Self::Patch => b"PATCH",
            Self::Post => b"POST",
            Self::Put => b"PUT",
            Self::Trace => b"TRACE",
        }
    }
}

fn is_tchar(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
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
    }

    #[test]
    fn parses_extension_method() {
        let method = Method::try_from_slice(b"PRI").unwrap();

        assert_eq!(method.as_slice(), b"PRI");
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
}
