//! HTTP/1.1 chunked transfer coding parsers.
//!
//! This crate exposes small composable parser functions for the chunk syntax from
//! [RFC 9112 §7.1] and [RFC 9112 §7.1.1].
//! It intentionally does not parse the optional `trailer-section`.
//!
//! [RFC 9112 §7.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
//! [RFC 9112 §7.1.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1

#![cfg_attr(docsrs, feature(doc_cfg))]

// TODO: move useful constructs, not just reexports from the rfc lib

pub use winnow_rfc9110::{parse_bws, parse_quoted_pair, parse_quoted_string, parse_token};
pub use winnow_rfc9112::{
    parse_chunk, parse_chunk_data, parse_chunk_ext, parse_chunk_ext_param, parse_chunk_ext_val,
    parse_chunk_header, parse_chunk_size, parse_last_chunk,
};
