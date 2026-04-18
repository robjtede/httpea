use core::ops::Range;

use winnow::{
    ascii::digit1,
    combinator::{alt, delimited, fail, opt, peek, preceded, repeat},
    prelude::*,
    stream::{AsChar, Compare, LocatingSlice, Location, Stream, StreamIsPartial},
    token::{literal, one_of, take_while},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RequestTargetIndices {
    Origin(RequestTargetOriginIndices),
    Absolute(RequestTargetAbsoluteIndices),
    Authority(RequestTargetAuthorityIndices),
    Asterisk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestTargetOriginIndices {
    pub(crate) path: Range<usize>,
    pub(crate) search: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestTargetAbsoluteIndices {
    pub(crate) scheme: Range<usize>,
    pub(crate) authority: Range<usize>,
    pub(crate) userinfo: Option<Range<usize>>,
    pub(crate) host: Range<usize>,
    pub(crate) port: Option<Range<usize>>,
    pub(crate) path: Range<usize>,
    pub(crate) search: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestTargetAuthorityIndices {
    pub(crate) authority: Range<usize>,
    pub(crate) host: Range<usize>,
    pub(crate) port: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthorityIndices {
    pub(crate) authority: Range<usize>,
    pub(crate) userinfo: Option<Range<usize>>,
    pub(crate) host: Range<usize>,
    pub(crate) port: Option<Range<usize>>,
}

/// # Request Line Examples
///
/// ```plain
/// GET /where?q=now HTTP/1.1
/// ```
///
/// # BNF
///
/// ```plain
/// origin-form   = absolute-path [ "?" query ]
/// absolute-path = 1*( "/" segment )
/// segment       = *pchar
/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
/// unreserved    = ALPHA / DIGIT / "-" / "." / "_" / "~"
/// pct-encoded   = "%" HEXDIG HEXDIG
/// sub-delims    = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
/// query         = *( pchar / "/" / "?" )
/// ```
pub(crate) fn parse_origin_form<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (parse_path, opt((b'?', parse_query)))
        .void()
        .parse_next(input)
}

pub(crate) fn parse_origin_form_indices(
    input: &mut LocatingSlice<&[u8]>,
) -> ModalResult<RequestTargetOriginIndices> {
    (parse_path.span(), opt((b'?', parse_query).span()))
        .map(|(path, search)| RequestTargetOriginIndices { path, search })
        .parse_next(input)
}

/// Parses a path.
///
/// Assumes entire input is a path, starting with a `/`, and does not include a query or fragment.
///
/// See:
/// - <https://datatracker.ietf.org/doc/html/rfc9112#name-syntax-notation>
/// - <https://datatracker.ietf.org/doc/html/rfc9110#name-uri-references>
/// - <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
pub(crate) fn parse_path<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    peek(b'/').parse_next(input)?;

    // absolute-path
    repeat(1.., (b'/', take_while(.., is_pchar)))
        .map(|()| ())
        .void()
        .parse_next(input)
}

/// Parses a query string.
///
/// Assumes entire input is only a query string without preceding `?` and without a rogue, trailing
/// fragement (i.e. `#`).
///
/// See:
/// - <https://datatracker.ietf.org/doc/html/rfc3986#section-3.4>
/// - <https://datatracker.ietf.org/doc/html/rfc3986#appendix-A>
pub(crate) fn parse_query<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    repeat(
        ..,
        one_of((
            is_pchar,
            // query literals
            [b'/', b'?'],
        )),
    )
    .map(|()| ())
    .void()
    .parse_next(input)
}

/// Returns `true` if the given character is in the `unreserved` group.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#appendix-A>
pub(crate) fn is_unreserved(char: char) -> bool {
    matches!(char, '0'..='9' | 'A'..='Z' | 'a'..='z' | '-' | '.' | '_' | '~')
}

/// Returns `true` if the given character is in the `sub-delims` group.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#appendix-A>
pub(crate) fn is_sub_delim(char: char) -> bool {
    matches!(
        char,
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
    )
}

/// Returns `true` if the given character is a valid `pchar`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
pub(crate) fn is_pchar(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char)
        || is_sub_delim(char)
        // pct-encoded
        || matches!(char, '%') // HEXDIG are included in `unreserved`; we do not validate hex escape sequences
        // pchar literals
        || matches!(char, ':' | '@')
}

/// Returns `true` if the given character is valid in `reg-name`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.2>
pub(crate) fn is_reg_name_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char)
        || is_sub_delim(char)
        // pct-encoded
        || matches!(char, '%') // HEXDIG are included in `unreserved`; we do not validate hex escape sequences
}

/// Returns `true` if the given character is valid in `userinfo`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.1>
pub(crate) fn is_userinfo_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char)
        || is_sub_delim(char)
        // pct-encoded
        || matches!(char, '%') // HEXDIG are included in `unreserved`; we do not validate hex escape sequences
        || matches!(char, ':')
}

/// Returns `true` if the given character is valid within an `IP-literal` body.
///
/// This covers the character groups referenced by the `IPv6address` and `IPvFuture`
/// productions from RFC 3986.
pub(crate) fn is_ip_literal_char(char: impl AsChar) -> bool {
    let char = char.as_char();

    is_unreserved(char)
        || is_sub_delim(char)
        // IPv6address / IPvFuture literals
        || matches!(char, ':')
}

/// Returns `true` if the given slice looks like an `IPv6address` or `IPvFuture` payload.
pub(crate) fn is_ip_literal_body(bytes: &[u8]) -> bool {
    bytes.contains(&b':') || matches!(bytes.first(), Some(b'v' | b'V')) && bytes.contains(&b'.')
}

/// # Request Line Examples
///
/// ```plain
/// CONNECT www.example.com:80 HTTP/1.1
/// ```
pub(crate) fn parse_authority_form<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (parse_uri_host, b':', parse_port).void().parse_next(input)
}

pub(crate) fn parse_authority_form_indices(
    input: &mut LocatingSlice<&[u8]>,
) -> ModalResult<RequestTargetAuthorityIndices> {
    let authority_start = input.current_token_start();

    let (host, _, port) = (parse_uri_host.span(), b':', parse_port.span()).parse_next(input)?;

    Ok(RequestTargetAuthorityIndices {
        authority: authority_start..input.current_token_start(),
        host,
        port,
    })
}

/// # Request Line Examples
///
/// ```plain
/// GET http://www.example.org/pub/WWW/TheProject.html HTTP/1.1
/// ```
///
/// # Policy
///
/// RFC 9112 defines `absolute-form = absolute-URI`, but this crate narrows absolute-form parsing
/// to the authority-based URI shape commonly used by HTTP-family schemes:
///
/// ```plain
/// scheme "://" authority path-abempty [ "?" query ]
/// ```
///
/// This rejects generic RFC 3986 absolute URIs like `htt:p//host` while still allowing arbitrary
/// schemes such as `git+http://...`.
pub(crate) fn parse_absolute_form<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (
        parse_scheme,
        b':',
        (b'/', b'/'),
        parse_authority,
        parse_path_abempty,
        opt((b'?', parse_query)),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn parse_absolute_form_indices(
    input: &mut LocatingSlice<&[u8]>,
) -> ModalResult<RequestTargetAbsoluteIndices> {
    (
        parse_scheme.span(),
        b':',
        (b'/', b'/'),
        parse_authority_indices,
        parse_path_abempty.span(),
        opt((b'?', parse_query).span()),
    )
        .map(
            |(scheme, _, _, authority, path, search)| RequestTargetAbsoluteIndices {
                scheme,
                authority: authority.authority,
                userinfo: authority.userinfo,
                host: authority.host,
                port: authority.port,
                path,
                search,
            },
        )
        .parse_next(input)
}

/// # Request Line Examples
///
/// ```plain
/// OPTIONS * HTTP/1.1
/// ```
pub(crate) fn parse_asterisk<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
{
    literal(b'*').void().parse_next(input)
}

pub(crate) fn parse_request_target<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((
        parse_asterisk,
        parse_origin_form,
        parse_authority_form,
        parse_absolute_form,
        fail,
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_request_target_indices(
    input: &mut LocatingSlice<&[u8]>,
) -> ModalResult<RequestTargetIndices> {
    alt((
        parse_asterisk.value(RequestTargetIndices::Asterisk),
        parse_origin_form_indices.map(RequestTargetIndices::Origin),
        parse_authority_form_indices.map(RequestTargetIndices::Authority),
        parse_absolute_form_indices.map(RequestTargetIndices::Absolute),
        fail,
    ))
    .parse_next(input)
}

/// Parses a `uri-host`.
///
/// RFC 9112 defines `authority-form = uri-host ":" port` and references the URI grammar for the
/// host production.
pub(crate) fn parse_uri_host<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((
        parse_ip_literal,
        // IPv4 addresses are valid in `reg-name` context
        parse_reg_name,
    ))
    .void()
    .parse_next(input)
}

/// Parses an `IP-literal`.
///
/// We enforce the surrounding brackets and restrict the inner character set to the `IPv6address` /
/// `IPvFuture` productions, while leaving the detailed numeric validation to a future pass.
pub(crate) fn parse_ip_literal<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    delimited(
        b'[',
        take_while(1.., is_ip_literal_char)
            .verify(|slice: &I::Slice| is_ip_literal_body(slice.as_ref())),
        b']',
    )
    .void()
    .parse_next(input)
}

/// Parses a `reg-name`.
///
/// RFC 3986 permits an empty `reg-name`, but `authority-form` requires a concrete `uri-host`, so
/// we require at least one character here.
pub(crate) fn parse_reg_name<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    take_while(1.., is_reg_name_char).void().parse_next(input)
}

/// Parses a `port`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.2.3>
pub(crate) fn parse_port<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: AsChar,
{
    digit1.void().parse_next(input)
}

/// Returns `true` if the given character is a valid first character in `scheme`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.1>
pub(crate) fn is_scheme_start(char: impl AsChar) -> bool {
    char.as_char().is_ascii_alphabetic()
}

/// Returns `true` if the given character is valid in `scheme`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.1>
pub(crate) fn is_scheme_char(char: impl AsChar) -> bool {
    matches!(char.as_char(), '0'..='9' | 'A'..='Z' | 'a'..='z' | '+' | '-' | '.')
}

/// Parses `scheme`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.1>
pub(crate) fn parse_scheme<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial,
    I::Token: Clone + AsChar,
{
    preceded(one_of(is_scheme_start), take_while(.., is_scheme_char))
        .void()
        .parse_next(input)
}

/// Parses `authority`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.2>
pub(crate) fn parse_authority<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    parse_authority_parts.void().parse_next(input)
}

pub(crate) fn parse_authority_parts<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (
        opt((take_while(1.., is_userinfo_char), b'@')),
        parse_uri_host,
        opt((b':', parse_port)),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn parse_authority_indices(
    input: &mut LocatingSlice<&[u8]>,
) -> ModalResult<AuthorityIndices> {
    let authority_start = input.current_token_start();

    let (userinfo, host, port) = (
        opt((take_while(1.., is_userinfo_char).span(), b'@').map(|(userinfo, _)| userinfo)),
        parse_uri_host.span(),
        opt(preceded(b':', parse_port.span())),
    )
        .parse_next(input)?;

    Ok(AuthorityIndices {
        authority: authority_start..input.current_token_start(),
        userinfo,
        host,
        port,
    })
}

/// Parses `path-abempty`.
///
/// See: <https://datatracker.ietf.org/doc/html/rfc3986#section-3.3>
pub(crate) fn parse_path_abempty<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    repeat(.., (b'/', take_while(.., is_pchar)))
        .map(|()| ())
        .void()
        .parse_next(input)
}

#[cfg(test)]
mod tests {
    use winnow::{
        BStr, Partial,
        error::{ErrMode, Needed},
    };

    use super::*;

    macro_rules! assert_backtrack {
        ($parser:expr, $input:expr $(,)?) => {
            assert!(
                matches!(
                    $parser.parse_peek(BStr::new($input)),
                    Err(ErrMode::Backtrack(_))
                ),
                "assertion failed: parser did not backtrack for input {:?}: {:?}",
                $input,
                $parser.parse_peek(BStr::new($input)),
            );
        };
    }

    macro_rules! assert_ok_remaining {
        ($parser:expr, $input:expr, $remaining:expr $(,)?) => {
            assert_eq!(
                $parser.parse_peek(BStr::new($input)),
                Ok((BStr::new($remaining), ())),
            );
        };
    }

    macro_rules! assert_partial_incomplete {
        ($parser:expr, $input:expr, $needed:expr $(,)?) => {
            assert_eq!(
                $parser.parse_peek(Partial::new(BStr::new($input))),
                Err(ErrMode::Incomplete($needed)),
            );
        };
    }

    #[test]
    fn validates_char_groups() {
        assert!(!is_unreserved('/'));
        assert!(is_unreserved('a'));
        assert!(is_unreserved('Z'));
        assert!(is_unreserved('0'));
        assert!(is_unreserved('~'));

        assert!(!is_sub_delim(':'));
        assert!(is_sub_delim('!'));
        assert!(is_sub_delim('='));

        assert!(!is_pchar(b'/'));
        assert!(is_pchar(b'='));
        assert!(is_pchar(b'%'));
        assert!(is_pchar(b':'));
        assert!(is_pchar(b'@'));

        assert!(!is_reg_name_char(b':'));
        assert!(!is_reg_name_char(b'@'));
        assert!(is_reg_name_char(b'%'));
        assert!(is_reg_name_char(b'.'));

        assert!(!is_ip_literal_char(b'['));
        assert!(!is_ip_literal_char(b'/'));
        assert!(is_ip_literal_char(b':'));
        assert!(is_ip_literal_char(b'v'));
        assert!(is_ip_literal_char(b'.'));
    }

    #[test]
    fn validates_ip_literal_bodies() {
        assert!(!is_ip_literal_body(b""));
        assert!(!is_ip_literal_body(b"localhost"));
        assert!(!is_ip_literal_body(b"v1"));
        assert!(!is_ip_literal_body(b"v1/"));

        assert!(is_ip_literal_body(b"::1"));
        assert!(is_ip_literal_body(b"2001:db8::1"));
        assert!(is_ip_literal_body(b"v1.future-host"));
        assert!(is_ip_literal_body(b"Vf.token:more"));
    }

    #[test]
    fn parses_reg_name() {
        assert_backtrack!(parse_reg_name, b"");
        assert_backtrack!(parse_reg_name, b"@localhost");

        assert_ok_remaining!(parse_reg_name, b"localhost", b"");
        assert_ok_remaining!(parse_reg_name, b"example.com:80", b":80");
        assert_ok_remaining!(parse_reg_name, b"xn--hllo-bpa.example", b"");
    }

    #[test]
    fn parses_ip_literal() {
        assert_backtrack!(parse_ip_literal, b"");
        assert_backtrack!(parse_ip_literal, b"[localhost]");
        assert_backtrack!(parse_ip_literal, b"[::1");

        assert_ok_remaining!(parse_ip_literal, b"[::1]", b"");
        assert_ok_remaining!(parse_ip_literal, b"[2001:db8::1]:443", b":443");
        assert_ok_remaining!(parse_ip_literal, b"[v1.future-host]", b"");
    }

    #[test]
    fn parses_uri_host() {
        assert_backtrack!(parse_uri_host, b"");
        assert_backtrack!(parse_uri_host, b"@localhost:80");

        assert_ok_remaining!(parse_uri_host, b"localhost:80", b":80");
        assert_ok_remaining!(parse_uri_host, b"127.0.0.1:80", b":80");
        assert_ok_remaining!(parse_uri_host, b"[::1]:80", b":80");
    }

    #[test]
    fn parses_scheme() {
        assert_backtrack!(parse_scheme, b"");
        assert_backtrack!(parse_scheme, b"1http");
        assert_partial_incomplete!(parse_scheme, b"", Needed::new(1));

        assert_ok_remaining!(parse_scheme, b"http", b"");
        assert_ok_remaining!(parse_scheme, b"https:", b":");
        assert_ok_remaining!(parse_scheme, b"http+unix:", b":");
    }

    #[test]
    fn parses_authority() {
        assert_backtrack!(parse_authority, b"");
        assert_backtrack!(parse_authority, b"@localhost");
        assert_partial_incomplete!(parse_authority, b"", Needed::new(1));

        assert_ok_remaining!(parse_authority, b"127.0.0.1:80", b"");
        assert_ok_remaining!(parse_authority, b"user:pass@localhost:3000", b"");
        assert_ok_remaining!(parse_authority, b"[::1]/path", b"/path");
    }

    #[test]
    fn parses_path_abempty() {
        assert_ok_remaining!(parse_path_abempty, b"", b"");
        assert_ok_remaining!(parse_path_abempty, b"/", b"");
        assert_ok_remaining!(parse_path_abempty, b"/foo/bar", b"");
        assert_ok_remaining!(parse_path_abempty, b"?foo=bar", b"?foo=bar");
    }

    #[test]
    fn parses_path() {
        assert_backtrack!(parse_path, b"");
        assert_partial_incomplete!(parse_path, b"", Needed::Unknown);
        assert_backtrack!(parse_path, b"=");

        assert_ok_remaining!(parse_path, b"/foo", b"");
        assert_ok_remaining!(parse_path, b"/foo/bar", b"");

        // parser assumes it won't receive a query but doesn't fail
        assert_ok_remaining!(parse_path, b"/foo/bar?baz", b"?baz");
    }

    #[test]
    fn parses_query() {
        assert_ok_remaining!(parse_query, b"", b"");
        assert_ok_remaining!(parse_query, b"=", b"");
        assert_ok_remaining!(parse_query, b"foo=bar", b"");
        assert_ok_remaining!(parse_query, b"foo=bar&baz", b"");
    }

    #[test]
    fn parses_authority_form() {
        assert_backtrack!(parse_authority_form, b"");
        assert_partial_incomplete!(parse_authority_form, b"", Needed::Unknown);

        assert_backtrack!(parse_authority_form, b"localhost");
        assert_backtrack!(parse_authority_form, b"user@localhost:3000");
        assert_backtrack!(parse_authority_form, b"[::1]");

        assert_ok_remaining!(parse_authority_form, b"localhost:3000", b"");
        assert_ok_remaining!(parse_authority_form, b"127.0.0.1:80", b"");
        assert_ok_remaining!(parse_authority_form, b"[::1]:443", b"");
    }

    #[test]
    fn parses_absolute_form() {
        assert_backtrack!(parse_absolute_form, b"");
        assert_backtrack!(parse_absolute_form, b"/foo");
        assert_backtrack!(parse_absolute_form, b"htt:p//host");
        assert_partial_incomplete!(parse_absolute_form, b"", Needed::new(1));

        assert_ok_remaining!(parse_absolute_form, b"http://127.0.0.1:61761/chunks", b"");
        assert_ok_remaining!(parse_absolute_form, b"https://127.0.0.1:61761", b"");
        assert_ok_remaining!(parse_absolute_form, b"http://127.0.0.1?foo=bar", b"");
        assert_ok_remaining!(parse_absolute_form, b"git+http://example.com/repo", b"");
    }

    #[test]
    fn parses_asterisk() {
        assert_backtrack!(parse_asterisk, b"");
        assert_partial_incomplete!(parse_asterisk, b"", Needed::Unknown);

        assert_ok_remaining!(parse_asterisk, b"*", b"");
        assert_ok_remaining!(parse_asterisk, b"**", b"*");
    }
}
