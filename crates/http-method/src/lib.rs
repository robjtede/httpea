//! HTTP method.

/// `GET` method.
pub const GET: Method = Method {
    repr: Repr::WellKnown(WellKnown::Get),
};

/// `POST` method.
pub const POST: Method = Method {
    repr: Repr::WellKnown(WellKnown::Post),
};

/// HTTP method.
#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    repr: Repr,
}

#[derive(Debug, Clone, PartialEq)]
enum Repr {
    WellKnown(WellKnown),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WellKnown {
    // Delete,
    Get,
    // Head,
    // Options,
    // Patch,
    Post,
    // Put,
    // Trace,
}
