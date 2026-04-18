//! Pimitive HTTP types.

#![cfg_attr(docsrs, feature(doc_cfg))]

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
        let _ = Field::try_from_slice(b"accept: application/json").unwrap();
        let _ = Method::try_from_slice(b"GET").unwrap();
        let _ = RequestTarget::try_from_slice(b"/").unwrap();
        let _ = StatusCode::from_u16(200);
        let _ = Version::try_from_slice(b"HTTP/1.1").unwrap();
    }
}
