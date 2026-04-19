//! HTTP/1.1 start-line, head-section, and trailer parsing helpers.

use core::{mem::MaybeUninit, ops::Range, slice};

use http_field::Field;
use http_method::Method;
use http_request_target::RequestTarget;
use http_status_code::StatusCode;
use http_version::Version;
use winnow::{prelude::*, stream::LocatingSlice};

mod parsing;

/// Prefix parsing status for streaming HTTP/1.1 input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseStatus<T> {
    /// More bytes are required before the parse can complete.
    Partial,

    /// Parsing completed successfully.
    Complete(T),
}

impl<T> ParseStatus<T> {
    /// Returns `true` when parsing completed successfully.
    #[inline]
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete(_))
    }

    /// Returns `true` when additional input is required.
    #[inline]
    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Partial)
    }

    /// Returns `true` when additional input is required before parsing can complete.
    #[inline]
    pub fn is_incomplete(&self) -> bool {
        self.is_partial()
    }

    /// Returns the parsed value if complete.
    #[inline]
    pub fn into_complete(self) -> Option<T> {
        match self {
            Self::Complete(value) => Some(value),
            Self::Partial => None,
        }
    }
}

/// Error returned when parsing an HTTP/1.1 request head fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseRequestHeadError {
    /// The request line was syntactically invalid.
    InvalidRequestLine,

    /// A header field line was syntactically invalid.
    InvalidHeaderField {
        /// Zero-based index within the header section.
        index: usize,
    },

    /// The caller-provided field buffer was too small to hold the header section.
    TooManyHeaderFields,
}

/// Error returned when parsing an HTTP/1.1 response head fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseResponseHeadError {
    /// The status line was syntactically invalid.
    InvalidStatusLine,

    /// A header field line was syntactically invalid.
    InvalidHeaderField {
        /// Zero-based index within the header section.
        index: usize,
    },

    /// The caller-provided field buffer was too small to hold the header section.
    TooManyHeaderFields,
}

/// Error returned when parsing a generic HTTP/1.1 head fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseHeadError {
    /// The request line was syntactically invalid.
    InvalidRequestLine,

    /// The status line was syntactically invalid.
    InvalidStatusLine,

    /// A header field line was syntactically invalid.
    InvalidHeaderField {
        /// Zero-based index within the header section.
        index: usize,
    },

    /// The caller-provided field buffer was too small to hold the header section.
    TooManyHeaderFields,
}

/// Error returned when parsing an HTTP/1.1 trailer field section fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseTrailerSectionError {
    /// A trailer field line was syntactically invalid.
    InvalidField {
        /// Zero-based index within the trailer field section.
        index: usize,
    },

    /// The caller-provided field buffer was too small to hold the trailer section.
    TooManyFields,
}

/// Error returned when parsing a single HTTP/1.1 trailer field line fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseTrailerFieldError {
    /// The provided input starts with the terminating empty line instead of a field line.
    EndOfSection,

    /// The trailer field line was syntactically invalid.
    InvalidField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldSectionError {
    InvalidField { index: usize },
    TooManyFields,
}

/// Parsed HTTP/1.1 request line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLine<'a> {
    inner: &'a [u8],
    method: Method,
    method_range: Range<usize>,
    target: RequestTarget<'a>,
    target_range: Range<usize>,
    version: Version,
    version_range: Range<usize>,
}

/// Parsed HTTP/1.1 status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine<'a> {
    inner: &'a [u8],
    version: Version,
    version_range: Range<usize>,
    status_code: StatusCode,
    status_code_range: Range<usize>,
    reason_phrase_range: Option<Range<usize>>,
}

/// Parsed HTTP/1.1 request head section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestHead<'a, Fields = Box<[Field<'a>]>> {
    head: &'a [u8],
    request_line: RequestLine<'a>,
    fields: Fields,
    remaining: &'a [u8],
}

/// Parsed HTTP/1.1 response head section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseHead<'a, Fields = Box<[Field<'a>]>> {
    head: &'a [u8],
    status_line: StatusLine<'a>,
    fields: Fields,
    remaining: &'a [u8],
}

/// Parsed HTTP/1.1 head section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head<'a, Fields = Box<[Field<'a>]>> {
    /// Request head section.
    Request(RequestHead<'a, Fields>),

    /// Response head section.
    Response(ResponseHead<'a, Fields>),
}

/// Parsed single HTTP field line terminated by CRLF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedField<'a> {
    line: &'a [u8],
    field: Field<'a>,
    remaining: &'a [u8],
}

/// Parsed trailer section, excluding the terminating empty line.
///
/// This matches the RFC 9112 `trailer-section` production. The required
/// trailing empty line remains in [`Self::remaining_bytes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrailerSection<'a, Fields = Box<[Field<'a>]>> {
    section: &'a [u8],
    fields: Fields,
    remaining: &'a [u8],
}

#[derive(Debug)]
struct ParsedFieldSection<'a, Fields> {
    section: &'a [u8],
    fields: Fields,
    remaining: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
enum EmptyLinePolicy {
    Consume,
    Leave,
}

impl<'a> RequestLine<'a> {
    /// Parses a request line from raw HTTP/1.1 bytes.
    pub fn try_from_slice(input: &'a [u8]) -> ModalResult<Self> {
        let mut located = LocatingSlice::new(input);
        let indices = parsing::parse_request_line_indices.parse_next(&mut located)?;
        let method = Method::try_from_slice(slice_range(input, &indices.method))
            .map_err(|_| unreachable!("request-line parser and method parser diverged"))?;
        let target = RequestTarget::try_from_slice(slice_range(input, &indices.target))
            .map_err(|_| unreachable!("request-line parser and request-target parser diverged"))?;
        let version = Version::try_from_slice(slice_range(input, &indices.version))
            .map_err(|_| unreachable!("request-line parser and version parser diverged"))?;

        Ok(Self {
            inner: input,
            method,
            method_range: indices.method,
            target,
            target_range: indices.target,
            version,
            version_range: indices.version,
        })
    }

    /// Returns the full request-line bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner
    }

    /// Returns the byte indices of the method component.
    #[inline]
    pub fn method_indices(&self) -> Range<usize> {
        self.method_range.clone()
    }

    /// Returns the parsed request method.
    #[inline]
    pub fn method(&self) -> Method {
        self.method.clone()
    }

    /// Returns the byte indices of the request-target component.
    #[inline]
    pub fn target_indices(&self) -> Range<usize> {
        self.target_range.clone()
    }

    /// Returns the parsed request-target.
    #[inline]
    pub fn target(&self) -> &RequestTarget<'a> {
        &self.target
    }

    /// Returns the byte indices of the HTTP-version component.
    #[inline]
    pub fn version_indices(&self) -> Range<usize> {
        self.version_range.clone()
    }

    /// Returns the parsed HTTP version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }
}

impl<'a> StatusLine<'a> {
    /// Parses a status line from raw HTTP/1.1 bytes.
    pub fn try_from_slice(input: &'a [u8]) -> ModalResult<Self> {
        let mut located = LocatingSlice::new(input);
        let indices = parsing::parse_status_line_indices.parse_next(&mut located)?;
        let version = Version::try_from_slice(slice_range(input, &indices.version))
            .map_err(|_| unreachable!("status-line parser and version parser diverged"))?;
        let status_code = parse_status_code(slice_range(input, &indices.status_code));

        Ok(Self {
            inner: input,
            version,
            version_range: indices.version,
            status_code,
            status_code_range: indices.status_code,
            reason_phrase_range: indices.reason_phrase,
        })
    }

    /// Returns the full status-line bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.inner
    }

    /// Returns the byte indices of the HTTP-version component.
    #[inline]
    pub fn version_indices(&self) -> Range<usize> {
        self.version_range.clone()
    }

    /// Returns the parsed HTTP version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Returns the byte indices of the status-code component.
    #[inline]
    pub fn status_code_indices(&self) -> Range<usize> {
        self.status_code_range.clone()
    }

    /// Returns the parsed status code.
    #[inline]
    pub fn status_code(&self) -> &StatusCode {
        &self.status_code
    }

    /// Returns the byte indices of the reason-phrase component.
    #[inline]
    pub fn reason_phrase_indices(&self) -> Option<Range<usize>> {
        self.reason_phrase_range.clone()
    }

    /// Returns the reason-phrase bytes.
    #[inline]
    pub fn reason_phrase(&self) -> Option<&'a [u8]> {
        optional_slice_range(self.inner, self.reason_phrase_range.as_ref())
    }
}

impl<'a, Fields> RequestHead<'a, Fields>
where
    Fields: AsRef<[Field<'a>]>,
{
    /// Returns the raw head section bytes, including the terminating empty line.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.head
    }

    /// Returns the number of bytes consumed by the head section.
    #[inline]
    pub fn consumed_len(&self) -> usize {
        self.head.len()
    }

    /// Returns the parsed request line.
    #[inline]
    pub fn request_line(&self) -> &RequestLine<'a> {
        &self.request_line
    }

    /// Returns the parsed header fields.
    #[inline]
    pub fn fields(&self) -> &[Field<'a>] {
        self.fields.as_ref()
    }

    /// Returns the bytes after the head section.
    #[inline]
    pub fn remaining_bytes(&self) -> &'a [u8] {
        self.remaining
    }
}

impl<'a, Fields> ResponseHead<'a, Fields>
where
    Fields: AsRef<[Field<'a>]>,
{
    /// Returns the raw head section bytes, including the terminating empty line.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.head
    }

    /// Returns the number of bytes consumed by the head section.
    #[inline]
    pub fn consumed_len(&self) -> usize {
        self.head.len()
    }

    /// Returns the parsed status line.
    #[inline]
    pub fn status_line(&self) -> &StatusLine<'a> {
        &self.status_line
    }

    /// Returns the parsed header fields.
    #[inline]
    pub fn fields(&self) -> &[Field<'a>] {
        self.fields.as_ref()
    }

    /// Returns the bytes after the head section.
    #[inline]
    pub fn remaining_bytes(&self) -> &'a [u8] {
        self.remaining
    }
}

impl<'a, Fields> Head<'a, Fields>
where
    Fields: AsRef<[Field<'a>]>,
{
    /// Returns the parsed request head if this is a request.
    #[inline]
    pub fn as_request(&self) -> Option<&RequestHead<'a, Fields>> {
        match self {
            Self::Request(head) => Some(head),
            Self::Response(_) => None,
        }
    }

    /// Returns the parsed response head if this is a response.
    #[inline]
    pub fn as_response(&self) -> Option<&ResponseHead<'a, Fields>> {
        match self {
            Self::Request(_) => None,
            Self::Response(head) => Some(head),
        }
    }
}

impl<'a> ParsedField<'a> {
    /// Returns the parsed field line bytes, excluding the trailing CRLF.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.line
    }

    /// Returns the number of bytes consumed by this field line, including CRLF.
    #[inline]
    pub fn consumed_len(&self) -> usize {
        self.line.len() + 2
    }

    /// Returns the parsed field.
    #[inline]
    pub fn field(&self) -> &Field<'a> {
        &self.field
    }

    /// Returns the bytes after the field line.
    #[inline]
    pub fn remaining_bytes(&self) -> &'a [u8] {
        self.remaining
    }
}

impl<'a, Fields> TrailerSection<'a, Fields>
where
    Fields: AsRef<[Field<'a>]>,
{
    /// Returns the raw trailer-section bytes.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.section
    }

    /// Returns the number of bytes consumed by the trailer-section.
    #[inline]
    pub fn consumed_len(&self) -> usize {
        self.section.len()
    }

    /// Returns the parsed trailer fields.
    #[inline]
    pub fn fields(&self) -> &[Field<'a>] {
        self.fields.as_ref()
    }

    /// Returns the bytes after the trailer-section.
    ///
    /// In a chunked body, this begins with the terminating empty line.
    #[inline]
    pub fn remaining_bytes(&self) -> &'a [u8] {
        self.remaining
    }
}

/// Parses an HTTP/1.1 request head from the front of `input` into a freshly allocated field slice.
pub fn parse_request_head_allocating(
    input: &[u8],
) -> Result<ParseStatus<RequestHead<'_>>, ParseRequestHeadError> {
    let Some(request_line_end) = parsing::find_crlf(input) else {
        return Ok(ParseStatus::Partial);
    };

    let request_line = RequestLine::try_from_slice(&input[..request_line_end])
        .map_err(|_| ParseRequestHeadError::InvalidRequestLine)?;
    let section_offset = request_line_end + 2;

    match parse_field_section_owned(&input[section_offset..], EmptyLinePolicy::Consume)
        .map_err(map_field_error_to_request_head_error)?
    {
        ParseStatus::Partial => Ok(ParseStatus::Partial),
        ParseStatus::Complete(section) => {
            let head_len = section_offset + section.section.len();

            Ok(ParseStatus::Complete(RequestHead {
                head: &input[..head_len],
                request_line,
                fields: section.fields,
                remaining: section.remaining,
            }))
        }
    }
}

/// Parses an HTTP/1.1 request head using a caller-provided field buffer.
pub fn parse_request_head<'buf, 'fields>(
    input: &'buf [u8],
    fields: &'fields mut [MaybeUninit<Field<'buf>>],
) -> Result<ParseStatus<RequestHead<'buf, &'fields [Field<'buf>]>>, ParseRequestHeadError> {
    let Some(request_line_end) = parsing::find_crlf(input) else {
        return Ok(ParseStatus::Partial);
    };

    let request_line = RequestLine::try_from_slice(&input[..request_line_end])
        .map_err(|_| ParseRequestHeadError::InvalidRequestLine)?;
    let section_offset = request_line_end + 2;

    match parse_field_section_with_uninit_fields(
        &input[section_offset..],
        fields,
        EmptyLinePolicy::Consume,
    )
    .map_err(map_field_error_to_request_head_error)?
    {
        ParseStatus::Partial => Ok(ParseStatus::Partial),
        ParseStatus::Complete(section) => {
            let head_len = section_offset + section.section.len();

            Ok(ParseStatus::Complete(RequestHead {
                head: &input[..head_len],
                request_line,
                fields: section.fields,
                remaining: section.remaining,
            }))
        }
    }
}

/// Parses an HTTP/1.1 response head from the front of `input` into a freshly allocated field slice.
pub fn parse_response_head_allocating(
    input: &[u8],
) -> Result<ParseStatus<ResponseHead<'_>>, ParseResponseHeadError> {
    let Some(status_line_end) = parsing::find_crlf(input) else {
        return Ok(ParseStatus::Partial);
    };

    let status_line = StatusLine::try_from_slice(&input[..status_line_end])
        .map_err(|_| ParseResponseHeadError::InvalidStatusLine)?;
    let section_offset = status_line_end + 2;

    match parse_field_section_owned(&input[section_offset..], EmptyLinePolicy::Consume)
        .map_err(map_field_error_to_response_head_error)?
    {
        ParseStatus::Partial => Ok(ParseStatus::Partial),
        ParseStatus::Complete(section) => {
            let head_len = section_offset + section.section.len();

            Ok(ParseStatus::Complete(ResponseHead {
                head: &input[..head_len],
                status_line,
                fields: section.fields,
                remaining: section.remaining,
            }))
        }
    }
}

/// Parses an HTTP/1.1 response head using a caller-provided field buffer.
pub fn parse_response_head<'buf, 'fields>(
    input: &'buf [u8],
    fields: &'fields mut [MaybeUninit<Field<'buf>>],
) -> Result<ParseStatus<ResponseHead<'buf, &'fields [Field<'buf>]>>, ParseResponseHeadError> {
    let Some(status_line_end) = parsing::find_crlf(input) else {
        return Ok(ParseStatus::Partial);
    };

    let status_line = StatusLine::try_from_slice(&input[..status_line_end])
        .map_err(|_| ParseResponseHeadError::InvalidStatusLine)?;
    let section_offset = status_line_end + 2;

    match parse_field_section_with_uninit_fields(
        &input[section_offset..],
        fields,
        EmptyLinePolicy::Consume,
    )
    .map_err(map_field_error_to_response_head_error)?
    {
        ParseStatus::Partial => Ok(ParseStatus::Partial),
        ParseStatus::Complete(section) => {
            let head_len = section_offset + section.section.len();

            Ok(ParseStatus::Complete(ResponseHead {
                head: &input[..head_len],
                status_line,
                fields: section.fields,
                remaining: section.remaining,
            }))
        }
    }
}

/// Parses an HTTP/1.1 head section from the front of `input` into a freshly allocated field slice.
pub fn parse_head_allocating(input: &[u8]) -> Result<ParseStatus<Head<'_>>, ParseHeadError> {
    if input.starts_with(b"HTTP/") {
        match parse_response_head_allocating(input) {
            Ok(ParseStatus::Partial) => Ok(ParseStatus::Partial),
            Ok(ParseStatus::Complete(head)) => Ok(ParseStatus::Complete(Head::Response(head))),
            Err(error) => Err(map_response_head_error(error)),
        }
    } else {
        match parse_request_head_allocating(input) {
            Ok(ParseStatus::Partial) => Ok(ParseStatus::Partial),
            Ok(ParseStatus::Complete(head)) => Ok(ParseStatus::Complete(Head::Request(head))),
            Err(error) => Err(map_request_head_error(error)),
        }
    }
}

/// Parses an HTTP/1.1 head section using a caller-provided field buffer.
pub fn parse_head<'buf, 'fields>(
    input: &'buf [u8],
    fields: &'fields mut [MaybeUninit<Field<'buf>>],
) -> Result<ParseStatus<Head<'buf, &'fields [Field<'buf>]>>, ParseHeadError> {
    if input.starts_with(b"HTTP/") {
        match parse_response_head(input, fields) {
            Ok(ParseStatus::Partial) => Ok(ParseStatus::Partial),
            Ok(ParseStatus::Complete(head)) => Ok(ParseStatus::Complete(Head::Response(head))),
            Err(error) => Err(map_response_head_error(error)),
        }
    } else {
        match parse_request_head(input, fields) {
            Ok(ParseStatus::Partial) => Ok(ParseStatus::Partial),
            Ok(ParseStatus::Complete(head)) => Ok(ParseStatus::Complete(Head::Request(head))),
            Err(error) => Err(map_request_head_error(error)),
        }
    }
}

/// Parses an HTTP/1.1 trailer section from the front of `input` into a freshly allocated field slice.
///
/// This parses the RFC 9112 `trailer-section` production and leaves the
/// terminating empty line in [`TrailerSection::remaining_bytes`].
pub fn parse_trailer_section_allocating(
    input: &[u8],
) -> Result<ParseStatus<TrailerSection<'_>>, ParseTrailerSectionError> {
    match parse_field_section_owned(input, EmptyLinePolicy::Leave)
        .map_err(map_field_error_to_trailer_section_error)?
    {
        ParseStatus::Partial => Ok(ParseStatus::Partial),
        ParseStatus::Complete(section) => Ok(ParseStatus::Complete(TrailerSection {
            section: section.section,
            fields: section.fields,
            remaining: section.remaining,
        })),
    }
}

/// Parses an HTTP/1.1 trailer section using a caller-provided field buffer.
pub fn parse_trailer_section<'buf, 'fields>(
    input: &'buf [u8],
    fields: &'fields mut [MaybeUninit<Field<'buf>>],
) -> Result<ParseStatus<TrailerSection<'buf, &'fields [Field<'buf>]>>, ParseTrailerSectionError> {
    match parse_field_section_with_uninit_fields(input, fields, EmptyLinePolicy::Leave)
        .map_err(map_field_error_to_trailer_section_error)?
    {
        ParseStatus::Partial => Ok(ParseStatus::Partial),
        ParseStatus::Complete(section) => Ok(ParseStatus::Complete(TrailerSection {
            section: section.section,
            fields: section.fields,
            remaining: section.remaining,
        })),
    }
}

/// Alias for [`parse_trailer_section_allocating`].
#[inline]
pub fn parse_footer_section_allocating(
    input: &[u8],
) -> Result<ParseStatus<TrailerSection<'_>>, ParseTrailerSectionError> {
    parse_trailer_section_allocating(input)
}

/// Alias for [`parse_trailer_section`].
#[inline]
pub fn parse_footer_section<'buf, 'fields>(
    input: &'buf [u8],
    fields: &'fields mut [MaybeUninit<Field<'buf>>],
) -> Result<ParseStatus<TrailerSection<'buf, &'fields [Field<'buf>]>>, ParseTrailerSectionError> {
    parse_trailer_section(input, fields)
}

/// Parses a single HTTP/1.1 trailer field line terminated by CRLF.
pub fn parse_trailer_field(
    input: &[u8],
) -> Result<ParseStatus<ParsedField<'_>>, ParseTrailerFieldError> {
    let Some(line_end) = parsing::find_crlf(input) else {
        return Ok(ParseStatus::Partial);
    };

    if line_end == 0 {
        return Err(ParseTrailerFieldError::EndOfSection);
    }

    let line = &input[..line_end];
    let field = Field::try_from_slice(line).map_err(|_| ParseTrailerFieldError::InvalidField)?;

    Ok(ParseStatus::Complete(ParsedField {
        line,
        field,
        remaining: &input[(line_end + 2)..],
    }))
}

/// Alias for [`parse_trailer_field`].
#[inline]
pub fn parse_footer_field(
    input: &[u8],
) -> Result<ParseStatus<ParsedField<'_>>, ParseTrailerFieldError> {
    parse_trailer_field(input)
}

fn parse_field_section_owned(
    input: &[u8],
    empty_line_policy: EmptyLinePolicy,
) -> Result<ParseStatus<ParsedFieldSection<'_, Box<[Field<'_>]>>>, FieldSectionError> {
    let mut fields = Vec::new();
    let mut offset = 0;
    let mut field_index = 0;

    loop {
        let Some(relative_line_end) = parsing::find_crlf(&input[offset..]) else {
            return Ok(ParseStatus::Partial);
        };
        let line_end = offset + relative_line_end;

        if line_end == offset {
            let consumed_len = match empty_line_policy {
                EmptyLinePolicy::Consume => line_end + 2,
                EmptyLinePolicy::Leave => line_end,
            };

            return Ok(ParseStatus::Complete(ParsedFieldSection {
                section: &input[..consumed_len],
                fields: fields.into_boxed_slice(),
                remaining: &input[consumed_len..],
            }));
        }

        let line = &input[offset..line_end];
        let field = Field::try_from_slice(line)
            .map_err(|_| FieldSectionError::InvalidField { index: field_index })?;

        fields.push(field);
        field_index += 1;
        offset = line_end + 2;
    }
}

fn parse_field_section_with_uninit_fields<'buf, 'fields>(
    input: &'buf [u8],
    fields: &'fields mut [MaybeUninit<Field<'buf>>],
    empty_line_policy: EmptyLinePolicy,
) -> Result<ParseStatus<ParsedFieldSection<'buf, &'fields [Field<'buf>]>>, FieldSectionError> {
    let mut offset = 0;
    let mut field_index = 0;

    loop {
        let Some(relative_line_end) = parsing::find_crlf(&input[offset..]) else {
            return Ok(ParseStatus::Partial);
        };
        let line_end = offset + relative_line_end;

        if line_end == offset {
            let consumed_len = match empty_line_policy {
                EmptyLinePolicy::Consume => line_end + 2,
                EmptyLinePolicy::Leave => line_end,
            };
            let initialized = unsafe { assume_init_fields_slice(&fields[..field_index]) };

            return Ok(ParseStatus::Complete(ParsedFieldSection {
                section: &input[..consumed_len],
                fields: initialized,
                remaining: &input[consumed_len..],
            }));
        }

        let line = &input[offset..line_end];
        let field = Field::try_from_slice(line)
            .map_err(|_| FieldSectionError::InvalidField { index: field_index })?;
        let Some(slot) = fields.get_mut(field_index) else {
            return Err(FieldSectionError::TooManyFields);
        };

        slot.write(field);
        field_index += 1;
        offset = line_end + 2;
    }
}

fn parse_status_code(input: &[u8]) -> StatusCode {
    let code =
        (input[0] - b'0') as u16 * 100 + (input[1] - b'0') as u16 * 10 + (input[2] - b'0') as u16;

    StatusCode::from_u16(code)
}

fn map_field_error_to_request_head_error(error: FieldSectionError) -> ParseRequestHeadError {
    match error {
        FieldSectionError::InvalidField { index } => {
            ParseRequestHeadError::InvalidHeaderField { index }
        }
        FieldSectionError::TooManyFields => ParseRequestHeadError::TooManyHeaderFields,
    }
}

fn map_field_error_to_response_head_error(error: FieldSectionError) -> ParseResponseHeadError {
    match error {
        FieldSectionError::InvalidField { index } => {
            ParseResponseHeadError::InvalidHeaderField { index }
        }
        FieldSectionError::TooManyFields => ParseResponseHeadError::TooManyHeaderFields,
    }
}

fn map_field_error_to_trailer_section_error(error: FieldSectionError) -> ParseTrailerSectionError {
    match error {
        FieldSectionError::InvalidField { index } => {
            ParseTrailerSectionError::InvalidField { index }
        }
        FieldSectionError::TooManyFields => ParseTrailerSectionError::TooManyFields,
    }
}

fn map_request_head_error(error: ParseRequestHeadError) -> ParseHeadError {
    match error {
        ParseRequestHeadError::InvalidRequestLine => ParseHeadError::InvalidRequestLine,
        ParseRequestHeadError::InvalidHeaderField { index } => {
            ParseHeadError::InvalidHeaderField { index }
        }
        ParseRequestHeadError::TooManyHeaderFields => ParseHeadError::TooManyHeaderFields,
    }
}

fn map_response_head_error(error: ParseResponseHeadError) -> ParseHeadError {
    match error {
        ParseResponseHeadError::InvalidStatusLine => ParseHeadError::InvalidStatusLine,
        ParseResponseHeadError::InvalidHeaderField { index } => {
            ParseHeadError::InvalidHeaderField { index }
        }
        ParseResponseHeadError::TooManyHeaderFields => ParseHeadError::TooManyHeaderFields,
    }
}

#[inline]
fn slice_range<'a>(input: &'a [u8], range: &Range<usize>) -> &'a [u8] {
    &input[range.start..range.end]
}

#[inline]
fn optional_slice_range<'a>(input: &'a [u8], range: Option<&Range<usize>>) -> Option<&'a [u8]> {
    range.map(|range| slice_range(input, range))
}

unsafe fn assume_init_fields_slice<'buf, 'fields>(
    fields: &'fields [MaybeUninit<Field<'buf>>],
) -> &'fields [Field<'buf>] {
    unsafe { slice::from_raw_parts(fields.as_ptr() as *const Field<'buf>, fields.len()) }
}

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use http_method::CONNECT;
    use http_status_code::StatusCode;

    use super::*;

    #[test]
    fn parses_request_line() {
        let line = RequestLine::try_from_slice(b"GET /where?q=now HTTP/1.1").unwrap();

        assert_eq!(line.as_bytes(), b"GET /where?q=now HTTP/1.1");
        assert_eq!(line.method_indices(), 0..3);
        assert_eq!(line.method().as_slice(), b"GET");
        assert_eq!(line.target_indices(), 4..16);
        assert_eq!(line.version_indices(), 17..25);
        assert_eq!(line.version(), Version::Http1_1);

        match line.target() {
            RequestTarget::Origin(target) => {
                assert_eq!(target.path(), b"/where");
                assert_eq!(target.query(), Some(&b"q=now"[..]));
            }
            other => panic!("expected origin-form target, got {:?}", other),
        }
    }

    #[test]
    fn parses_connect_request_line() {
        let line = RequestLine::try_from_slice(b"CONNECT www.example.com:443 HTTP/1.1").unwrap();

        assert_eq!(line.method(), CONNECT);
        assert_eq!(line.version(), Version::Http1_1);

        match line.target() {
            RequestTarget::Authority(target) => {
                assert_eq!(target.host(), b"www.example.com");
                assert_eq!(target.port(), b"443");
            }
            other => panic!("expected authority-form target, got {:?}", other),
        }
    }

    #[test]
    fn rejects_invalid_request_lines() {
        assert!(RequestLine::try_from_slice(b"GET  / HTTP/1.1").is_err());
        assert!(RequestLine::try_from_slice(b"GET /\r HTTP/1.1").is_err());
        assert!(RequestLine::try_from_slice(b"GET / HTTP/2").is_err());
        assert!(RequestLine::try_from_slice(b"GET / HTTP/1.1\r\n").is_err());
    }

    #[test]
    fn parses_status_line() {
        let line = StatusLine::try_from_slice(b"HTTP/1.1 200 OK").unwrap();

        assert_eq!(line.as_bytes(), b"HTTP/1.1 200 OK");
        assert_eq!(line.version(), Version::Http1_1);
        assert_eq!(line.version_indices(), 0..8);
        assert_eq!(line.status_code_indices(), 9..12);
        assert_eq!(line.status_code(), &StatusCode::from_u16(200));
        assert_eq!(line.reason_phrase_indices(), Some(13..15));
        assert_eq!(line.reason_phrase(), Some(&b"OK"[..]));
    }

    #[test]
    fn parses_status_line_with_empty_reason_phrase() {
        let line = StatusLine::try_from_slice(b"HTTP/1.1 204 ").unwrap();

        assert_eq!(line.status_code(), &StatusCode::from_u16(204));
        assert_eq!(line.reason_phrase(), None);
    }

    #[test]
    fn rejects_invalid_status_lines() {
        assert!(StatusLine::try_from_slice(b"HTTP/1.1 20 OK").is_err());
        assert!(StatusLine::try_from_slice(b"HTTP/2 200 OK").is_err());
        assert!(StatusLine::try_from_slice(b"HTTP/1.1 200").is_err());
    }

    #[test]
    fn parses_request_head_without_headers() {
        let head = parse_request_head_allocating(b"GET / HTTP/1.1\r\n\r\n")
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(head.as_bytes(), b"GET / HTTP/1.1\r\n\r\n");
        assert_eq!(head.consumed_len(), 18);
        assert!(head.fields().is_empty());
        assert_eq!(head.remaining_bytes(), b"");
    }

    #[test]
    fn parses_request_head_with_headers_and_body_remainder() {
        let input =
            b"POST /upload HTTP/1.1\r\nhost: example.com\r\ncontent-length: 4\r\n\r\n\x00\xff\x10\x7f";
        let head = parse_request_head_allocating(input)
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(head.request_line().method().as_slice(), b"POST");
        assert_eq!(head.fields().len(), 2);
        assert_eq!(head.fields()[0].name().as_slice(), b"host");
        assert_eq!(head.fields()[0].value().as_slice(), b"example.com");
        assert_eq!(head.fields()[1].name().as_slice(), b"content-length");
        assert_eq!(head.fields()[1].value().as_slice(), b"4");
        assert_eq!(head.remaining_bytes(), b"\x00\xff\x10\x7f");
        assert_eq!(&input[..head.consumed_len()], head.as_bytes());
    }

    #[test]
    fn parses_request_head_with_uninit_fields() {
        let input = b"GET / HTTP/1.1\r\nhost: example.com\r\naccept: */*\r\n\r\nbody";
        let mut fields: [MaybeUninit<Field<'_>>; 2] = [const { MaybeUninit::uninit() }; 2];
        let head = parse_request_head(input, &mut fields)
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(head.fields().len(), 2);
        assert_eq!(head.fields()[0].name().as_slice(), b"host");
        assert_eq!(head.fields()[1].value().as_slice(), b"*/*");
        assert_eq!(head.remaining_bytes(), b"body");
    }

    #[test]
    fn request_head_with_uninit_fields_rejects_too_many_fields() {
        let input = b"GET / HTTP/1.1\r\nhost: example.com\r\naccept: */*\r\n\r\n";
        let mut fields: [MaybeUninit<Field<'_>>; 1] = [const { MaybeUninit::uninit() }; 1];

        assert_eq!(
            parse_request_head(input, &mut fields),
            Err(ParseRequestHeadError::TooManyHeaderFields),
        );
    }

    #[test]
    fn parses_response_head_with_headers() {
        let input = b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nserver: httpea\r\n\r\nbody";
        let head = parse_response_head_allocating(input)
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(head.status_line().status_code(), &StatusCode::from_u16(200));
        assert_eq!(head.fields().len(), 2);
        assert_eq!(head.fields()[0].name().as_slice(), b"content-length");
        assert_eq!(head.remaining_bytes(), b"body");
    }

    #[test]
    fn parses_response_head_with_empty_reason_phrase() {
        let head = parse_response_head_allocating(b"HTTP/1.1 204 \r\n\r\n")
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(head.status_line().reason_phrase(), None);
        assert!(head.fields().is_empty());
    }

    #[test]
    fn parses_response_head_with_uninit_fields() {
        let input = b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nserver: httpea\r\n\r\nbody";
        let mut fields: [MaybeUninit<Field<'_>>; 2] = [const { MaybeUninit::uninit() }; 2];
        let head = parse_response_head(input, &mut fields)
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(head.fields().len(), 2);
        assert_eq!(head.fields()[1].name().as_slice(), b"server");
    }

    #[test]
    fn parses_generic_request_head() {
        let head = parse_head_allocating(b"GET / HTTP/1.1\r\nhost: example.com\r\n\r\n")
            .unwrap()
            .into_complete()
            .unwrap();

        assert!(matches!(head, Head::Request(_)));
        assert_eq!(
            head.as_request().unwrap().fields()[0].value().as_slice(),
            b"example.com",
        );
    }

    #[test]
    fn parses_generic_response_head() {
        let head =
            parse_head_allocating(b"HTTP/1.1 503 Service Unavailable\r\nretry-after: 60\r\n\r\n")
                .unwrap()
                .into_complete()
                .unwrap();

        assert!(matches!(head, Head::Response(_)));
        assert_eq!(
            head.as_response().unwrap().status_line().status_code(),
            &StatusCode::from_u16(503),
        );
    }

    #[test]
    fn parses_generic_head_with_uninit_fields() {
        let input = b"HTTP/1.1 503 Service Unavailable\r\nretry-after: 60\r\n\r\n";
        let mut fields: [MaybeUninit<Field<'_>>; 1] = [const { MaybeUninit::uninit() }; 1];
        let head = parse_head(input, &mut fields)
            .unwrap()
            .into_complete()
            .unwrap();

        assert!(matches!(head, Head::Response(_)));
        assert_eq!(
            head.as_response().unwrap().fields()[0].name().as_slice(),
            b"retry-after"
        );
    }

    #[test]
    fn request_head_is_partial_without_request_line_crlf() {
        assert_eq!(
            parse_request_head_allocating(b"GET / HTTP/1.1").unwrap(),
            ParseStatus::Partial,
        );
    }

    #[test]
    fn response_head_is_partial_without_terminating_blank_line() {
        assert_eq!(
            parse_response_head_allocating(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n").unwrap(),
            ParseStatus::Partial,
        );
    }

    #[test]
    fn generic_head_is_partial_without_start_line_crlf() {
        assert_eq!(
            parse_head_allocating(b"HTTP/1.1 200 OK").unwrap(),
            ParseStatus::Partial
        );
    }

    #[test]
    fn rejects_invalid_request_head_request_line() {
        assert_eq!(
            parse_request_head_allocating(b"GET  / HTTP/1.1\r\n\r\n"),
            Err(ParseRequestHeadError::InvalidRequestLine),
        );
    }

    #[test]
    fn rejects_invalid_response_head_status_line() {
        assert_eq!(
            parse_response_head_allocating(b"HTTP/1.1 20 OK\r\n\r\n"),
            Err(ParseResponseHeadError::InvalidStatusLine),
        );
    }

    #[test]
    fn rejects_invalid_header_field() {
        assert_eq!(
            parse_request_head_allocating(b"GET / HTTP/1.1\r\nbad name: value\r\n\r\n"),
            Err(ParseRequestHeadError::InvalidHeaderField { index: 0 }),
        );
    }

    #[test]
    fn rejects_obs_fold_header_continuation() {
        assert_eq!(
            parse_response_head_allocating(b"HTTP/1.1 200 OK\r\nx-test: one\r\n two\r\n\r\n"),
            Err(ParseResponseHeadError::InvalidHeaderField { index: 1 }),
        );
    }

    #[test]
    fn parses_empty_trailer_section_and_leaves_final_crlf() {
        let trailers = parse_trailer_section_allocating(b"\r\nnext")
            .unwrap()
            .into_complete()
            .unwrap();

        assert!(trailers.fields().is_empty());
        assert_eq!(trailers.as_bytes(), b"");
        assert_eq!(trailers.remaining_bytes(), b"\r\nnext");
    }

    #[test]
    fn parses_trailer_section_with_multiple_fields() {
        let trailers = parse_trailer_section_allocating(b"etag: abc\r\nexpires: now\r\n\r\nrest")
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(trailers.fields().len(), 2);
        assert_eq!(trailers.fields()[0].name().as_slice(), b"etag");
        assert_eq!(trailers.fields()[1].value().as_slice(), b"now");
        assert_eq!(trailers.as_bytes(), b"etag: abc\r\nexpires: now\r\n");
        assert_eq!(trailers.remaining_bytes(), b"\r\nrest");
    }

    #[test]
    fn parses_trailer_section_with_uninit_fields() {
        let mut fields: [MaybeUninit<Field<'_>>; 2] = [const { MaybeUninit::uninit() }; 2];
        let trailers = parse_trailer_section(b"etag: abc\r\nexpires: now\r\n\r\nrest", &mut fields)
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(trailers.fields().len(), 2);
        assert_eq!(trailers.fields()[1].name().as_slice(), b"expires");
        assert_eq!(trailers.remaining_bytes(), b"\r\nrest");
    }

    #[test]
    fn trailer_section_with_uninit_fields_rejects_too_many_fields() {
        let mut fields: [MaybeUninit<Field<'_>>; 1] = [const { MaybeUninit::uninit() }; 1];

        assert_eq!(
            parse_trailer_section(b"etag: abc\r\nexpires: now\r\n\r\n", &mut fields,),
            Err(ParseTrailerSectionError::TooManyFields),
        );
    }

    #[test]
    fn trailer_section_is_partial_without_delimiting_empty_line() {
        assert_eq!(
            parse_trailer_section_allocating(b"etag: abc\r\nexpires: now\r\n").unwrap(),
            ParseStatus::Partial,
        );
    }

    #[test]
    fn trailer_section_is_partial_with_incomplete_field_line() {
        assert_eq!(
            parse_trailer_section_allocating(b"etag: abc\r\nexpires: now\r").unwrap(),
            ParseStatus::Partial,
        );
    }

    #[test]
    fn rejects_invalid_trailer_field_in_section() {
        assert_eq!(
            parse_trailer_section_allocating(b"etag: abc\r\nbad name: value\r\n\r\n"),
            Err(ParseTrailerSectionError::InvalidField { index: 1 }),
        );
    }

    #[test]
    fn parses_single_trailer_field() {
        let parsed = parse_trailer_field(b"etag: abc\r\nrest")
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(parsed.as_bytes(), b"etag: abc");
        assert_eq!(parsed.consumed_len(), 11);
        assert_eq!(parsed.field().name().as_slice(), b"etag");
        assert_eq!(parsed.field().value().as_slice(), b"abc");
        assert_eq!(parsed.remaining_bytes(), b"rest");
    }

    #[test]
    fn trailer_field_is_partial_without_crlf() {
        assert_eq!(
            parse_trailer_field(b"etag: abc").unwrap(),
            ParseStatus::Partial,
        );
    }

    #[test]
    fn trailer_field_rejects_end_of_section() {
        assert_eq!(
            parse_trailer_field(b"\r\n"),
            Err(ParseTrailerFieldError::EndOfSection),
        );
    }

    #[test]
    fn trailer_field_rejects_invalid_line() {
        assert_eq!(
            parse_trailer_field(b"bad name: value\r\n"),
            Err(ParseTrailerFieldError::InvalidField),
        );
    }

    #[test]
    fn footer_aliases_match_trailer_functions() {
        let field = parse_footer_field(b"etag: abc\r\n")
            .unwrap()
            .into_complete()
            .unwrap();
        let mut footer_fields: [MaybeUninit<Field<'_>>; 1] = [const { MaybeUninit::uninit() }; 1];
        let section = parse_footer_section(b"etag: abc\r\n\r\n", &mut footer_fields)
            .unwrap()
            .into_complete()
            .unwrap();

        assert_eq!(field.field().name().as_slice(), b"etag");
        assert_eq!(section.fields()[0].value().as_slice(), b"abc");
        assert_eq!(section.remaining_bytes(), b"\r\n");
    }
}
