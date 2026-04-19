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
