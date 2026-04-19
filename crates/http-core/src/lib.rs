//! Pimitive HTTP types.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub use http_chunked::{
    parse_bws, parse_chunk, parse_chunk_data, parse_chunk_ext, parse_chunk_ext_param,
    parse_chunk_ext_val, parse_chunk_header, parse_chunk_size, parse_last_chunk, parse_quoted_pair,
    parse_quoted_string, parse_token,
};
pub use http_field::{Field, FieldName, FieldValue};
pub use http_method::Method;
pub use http_request_target::{
    RequestTarget, RequestTargetAbsolute, RequestTargetAuthority, RequestTargetOrigin,
};
pub use http_status_code::StatusCode;
pub use http_version::Version;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn re_exports_primary_types() {
        let mut chunk = &b"1\r\na\r\n"[..];
        let mut last_chunk = &b"0\r\n"[..];

        let _ = parse_chunk(&mut chunk).unwrap();
        parse_last_chunk(&mut last_chunk).unwrap();
        let _ = Field::try_from_slice(b"accept: application/json").unwrap();
        let _ = Method::try_from_slice(b"GET").unwrap();
        let _ = RequestTarget::try_from_slice(b"/").unwrap();
        let _ = StatusCode::from_u16(200);
        let _ = Version::try_from_slice(b"HTTP/1.1").unwrap();
    }
}
