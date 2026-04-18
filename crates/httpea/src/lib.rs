#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "http-1_1")]
pub mod http_1_1;

#[cfg(feature = "http-1_1")]
pub use http_1_1::RequestLine;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[cfg(feature = "http-1_1")]
    #[test]
    fn re_exports_request_line() {
        let _ = RequestLine::try_from_slice(b"GET / HTTP/1.1").unwrap();
    }
}
