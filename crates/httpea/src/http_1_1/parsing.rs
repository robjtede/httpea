use core::ops::Range;

use http_method::Method;
use http_request_target::RequestTarget;
use http_version::Version;
use winnow::{
    combinator::eof,
    error::{ContextError, ErrMode, ParserError},
    prelude::*,
    stream::LocatingSlice,
    token::take_while,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestLineIndices {
    pub(crate) method: Range<usize>,
    pub(crate) target: Range<usize>,
    pub(crate) version: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusLineIndices {
    pub(crate) version: Range<usize>,
    pub(crate) status_code: Range<usize>,
    pub(crate) reason_phrase: Option<Range<usize>>,
}

pub(crate) fn parse_request_line_indices(
    input: &mut LocatingSlice<&[u8]>,
) -> ModalResult<RequestLineIndices> {
    let method = parse_method_component.span().parse_next(input)?;
    b' '.parse_next(input)?;
    let target = parse_target_component.span().parse_next(input)?;
    b' '.parse_next(input)?;
    let version = parse_version_component.span().parse_next(input)?;
    eof.parse_next(input)?;

    Ok(RequestLineIndices {
        method,
        target,
        version,
    })
}

pub(crate) fn parse_status_line_indices(
    input: &mut LocatingSlice<&[u8]>,
) -> ModalResult<StatusLineIndices> {
    let version = parse_version_component.span().parse_next(input)?;
    b' '.parse_next(input)?;
    let status_code = parse_status_code_component.span().parse_next(input)?;
    b' '.parse_next(input)?;
    let reason_phrase = parse_reason_phrase_component.span().parse_next(input).ok();
    eof.parse_next(input)?;

    Ok(StatusLineIndices {
        version,
        status_code,
        reason_phrase,
    })
}

fn parse_method_component(input: &mut LocatingSlice<&[u8]>) -> ModalResult<()> {
    let slice = take_while(1.., is_request_line_visible_byte).parse_next(input)?;

    if Method::try_from_slice(slice).is_ok() {
        Ok(())
    } else {
        Err(<ErrMode<ContextError> as ParserError<
            LocatingSlice<&[u8]>,
        >>::from_input(input))
    }
}

fn parse_target_component(input: &mut LocatingSlice<&[u8]>) -> ModalResult<()> {
    let slice = take_while(1.., is_request_line_visible_byte).parse_next(input)?;

    if RequestTarget::try_from_slice(slice).is_ok() {
        Ok(())
    } else {
        Err(<ErrMode<ContextError> as ParserError<
            LocatingSlice<&[u8]>,
        >>::from_input(input))
    }
}

fn parse_version_component(input: &mut LocatingSlice<&[u8]>) -> ModalResult<()> {
    let slice = take_while(1.., is_request_line_visible_byte).parse_next(input)?;

    match Version::try_from_slice(slice) {
        Ok(Version::Http1_0 | Version::Http1_1) => Ok(()),
        _ => Err(<ErrMode<ContextError> as ParserError<
            LocatingSlice<&[u8]>,
        >>::from_input(input)),
    }
}

fn parse_status_code_component(input: &mut LocatingSlice<&[u8]>) -> ModalResult<()> {
    let slice = take_while(3..=3, |byte: u8| byte.is_ascii_digit()).parse_next(input)?;
    let code =
        (slice[0] - b'0') as u16 * 100 + (slice[1] - b'0') as u16 * 10 + (slice[2] - b'0') as u16;

    if (100..=999).contains(&code) {
        Ok(())
    } else {
        Err(<ErrMode<ContextError> as ParserError<
            LocatingSlice<&[u8]>,
        >>::from_input(input))
    }
}

fn parse_reason_phrase_component(input: &mut LocatingSlice<&[u8]>) -> ModalResult<()> {
    take_while(1.., is_reason_phrase_byte)
        .void()
        .parse_next(input)
}

fn is_request_line_visible_byte(byte: u8) -> bool {
    !matches!(byte, b' ' | b'\r' | b'\n')
}

fn is_reason_phrase_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b' ' | 0x21..=0x7E | 0x80..=0xFF)
}

pub(crate) fn find_crlf(input: &[u8]) -> Option<usize> {
    input.windows(2).position(|window| window == b"\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_request_line_indices() {
        let indices =
            parse_request_line_indices(&mut LocatingSlice::new(&b"GET /items?q=1 HTTP/1.1"[..]))
                .unwrap();

        assert_eq!(indices.method, 0..3);
        assert_eq!(indices.target, 4..14);
        assert_eq!(indices.version, 15..23);
    }

    #[test]
    fn rejects_invalid_request_line_components() {
        assert!(
            parse_request_line_indices(&mut LocatingSlice::new(&b"GE T / HTTP/1.1"[..])).is_err()
        );
        assert!(
            parse_request_line_indices(&mut LocatingSlice::new(&b"GET localhost HTTP/1.1"[..]))
                .is_err()
        );
        assert!(parse_request_line_indices(&mut LocatingSlice::new(&b"GET / HTTP/2"[..])).is_err());
    }

    #[test]
    fn parses_status_line_indices() {
        let indices =
            parse_status_line_indices(&mut LocatingSlice::new(&b"HTTP/1.1 200 OK"[..])).unwrap();
        assert_eq!(indices.version, 0..8);
        assert_eq!(indices.status_code, 9..12);
        assert_eq!(indices.reason_phrase, Some(13..15));

        let indices =
            parse_status_line_indices(&mut LocatingSlice::new(&b"HTTP/1.0 204 "[..])).unwrap();
        assert_eq!(indices.version, 0..8);
        assert_eq!(indices.status_code, 9..12);
        assert_eq!(indices.reason_phrase, None);
    }

    #[test]
    fn rejects_invalid_status_lines() {
        assert!(parse_status_line_indices(&mut LocatingSlice::new(&b"HTTP/2 200 OK"[..])).is_err());
        assert!(
            parse_status_line_indices(&mut LocatingSlice::new(&b"HTTP/1.1 00 OK"[..])).is_err()
        );
        assert!(
            parse_status_line_indices(&mut LocatingSlice::new(&b"HTTP/1.1 200 OK\r"[..])).is_err()
        );
    }

    #[test]
    fn validates_component_parsers() {
        assert!(parse_method_component(&mut LocatingSlice::new(&b"GET"[..])).is_ok());
        assert!(parse_method_component(&mut LocatingSlice::new(&b"\r"[..])).is_err());

        assert!(parse_target_component(&mut LocatingSlice::new(&b"/items"[..])).is_ok());
        assert!(parse_target_component(&mut LocatingSlice::new(&b"?"[..])).is_err());

        assert!(parse_version_component(&mut LocatingSlice::new(&b"HTTP/1.1"[..])).is_ok());
        assert!(parse_version_component(&mut LocatingSlice::new(&b"HTTP/3"[..])).is_err());

        assert!(parse_status_code_component(&mut LocatingSlice::new(&b"200"[..])).is_ok());
        assert!(parse_status_code_component(&mut LocatingSlice::new(&b"00"[..])).is_err());

        assert!(parse_reason_phrase_component(&mut LocatingSlice::new(&b"OK\t\x80"[..])).is_ok());
        assert!(parse_reason_phrase_component(&mut LocatingSlice::new(&b"\r"[..])).is_err());
    }

    #[test]
    fn validates_byte_classifiers_and_crlf_search() {
        assert!(is_request_line_visible_byte(b'G'));
        assert!(!is_request_line_visible_byte(b' '));
        assert!(!is_request_line_visible_byte(b'\r'));

        assert!(is_reason_phrase_byte(b' '));
        assert!(is_reason_phrase_byte(b'\t'));
        assert!(is_reason_phrase_byte(0x80));
        assert!(!is_reason_phrase_byte(b'\n'));

        assert_eq!(find_crlf(b""), None);
        assert_eq!(find_crlf(b"abc"), None);
        assert_eq!(find_crlf(b"abc\r\ndef"), Some(3));
    }
}
