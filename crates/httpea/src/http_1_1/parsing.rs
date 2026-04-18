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

fn is_request_line_visible_byte(byte: u8) -> bool {
    !matches!(byte, b' ' | b'\r' | b'\n')
}
