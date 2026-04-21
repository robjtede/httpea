use core::ops::Range;

use winnow::{
    combinator::{alt, fail, opt, preceded},
    prelude::*,
    stream::{AsChar, Compare, Location, Stream, StreamIsPartial},
    token::{literal, take_while},
};
use winnow_rfc3986::{
    is_userinfo_char, parse_authority, parse_path, parse_path_abempty, parse_port, parse_query,
    parse_scheme, parse_uri_host,
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
#[inline]
pub(crate) fn parse_origin_form<I>(input: &mut I) -> ModalResult<RequestTargetOriginIndices>
where
    I: Stream<Token = u8> + StreamIsPartial + Location + Compare<u8>,
{
    (parse_path.span(), opt((b'?', parse_query).span()))
        .map(|(path, search)| RequestTargetOriginIndices { path, search })
        .parse_next(input)
}

#[allow(dead_code)]
fn parse_origin_form_only<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
{
    (parse_path, opt((b'?', parse_query)))
        .void()
        .parse_next(input)
}

/// # Request Line Examples
///
/// ```plain
/// CONNECT www.example.com:80 HTTP/1.1
/// ```
///
/// # BNF
///
/// ```plain
/// authority-form = uri-host ":" port
/// ```
///
/// See: [RFC 9112 §3.2](https://datatracker.ietf.org/doc/html/rfc9112#name-connect)
#[inline]
pub(crate) fn parse_authority_form<I>(input: &mut I) -> ModalResult<RequestTargetAuthorityIndices>
where
    I: Stream<Token = u8> + StreamIsPartial + Location + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    let authority_start = input.current_token_start();

    let (host, _, port) = (parse_uri_host.span(), b':', parse_port.span()).parse_next(input)?;

    Ok(RequestTargetAuthorityIndices {
        authority: authority_start..input.current_token_start(),
        host,
        port,
    })
}

#[allow(dead_code)]
fn parse_authority_form_only<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    (parse_uri_host, b':', parse_port).void().parse_next(input)
}

/// # Request Line Examples
///
/// ```plain
/// GET http://www.example.org/pub/WWW/TheProject.html HTTP/1.1
/// ```
///
/// # Policy
///
/// [RFC 9112](https://datatracker.ietf.org/doc/html/rfc9112) defines
/// `absolute-form = absolute-URI`, but this crate narrows absolute-form parsing to the
/// authority-based URI shape commonly used by HTTP-family schemes:
///
/// ```plain
/// scheme "://" authority path-abempty [ "?" query ]
/// ```
///
/// This rejects generic [RFC 3986](https://datatracker.ietf.org/doc/html/rfc3986)
/// absolute URIs like `htt:p//host` while still allowing arbitrary schemes such as
/// `git+http://...`.
#[inline]
pub(crate) fn parse_absolute_form<I>(input: &mut I) -> ModalResult<RequestTargetAbsoluteIndices>
where
    I: Stream<Token = u8> + StreamIsPartial + Location + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
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

#[allow(dead_code)]
fn parse_absolute_form_only<I>(input: &mut I) -> ModalResult<()>
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

/// # Request Line Examples
///
/// ```plain
/// OPTIONS * HTTP/1.1
/// ```
#[inline]
pub(crate) fn parse_asterisk<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
{
    literal(b'*').void().parse_next(input)
}

#[allow(dead_code)]
fn parse_request_target_only<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream + StreamIsPartial + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((
        parse_asterisk,
        parse_origin_form_only,
        parse_authority_form_only,
        parse_absolute_form_only,
        fail,
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_request_target<I>(input: &mut I) -> ModalResult<RequestTargetIndices>
where
    I: Stream<Token = u8> + StreamIsPartial + Location + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
    alt((
        parse_asterisk.value(RequestTargetIndices::Asterisk),
        parse_origin_form.map(RequestTargetIndices::Origin),
        parse_authority_form.map(RequestTargetIndices::Authority),
        parse_absolute_form.map(RequestTargetIndices::Absolute),
        fail,
    ))
    .parse_next(input)
}

#[inline]
#[allow(dead_code)]
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

#[inline]
pub(crate) fn parse_authority_indices<I>(input: &mut I) -> ModalResult<AuthorityIndices>
where
    I: Stream<Token = u8> + StreamIsPartial + Location + Compare<u8>,
    I::Token: Clone + AsChar,
    I::Slice: AsRef<[u8]>,
{
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

#[cfg(test)]
mod tests {
    use winnow::{
        BStr, Partial,
        error::{ErrMode, Needed},
        stream::LocatingSlice,
    };
    use winnow_rfc3986::{
        is_ip_literal_body, is_ip_literal_char, is_pchar, is_reg_name_char, is_sub_delim,
        is_unreserved, parse_authority, parse_ip_literal, parse_path, parse_query, parse_reg_name,
        parse_scheme, parse_uri_host,
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
            assert!(
                matches!(
                    $parser.parse_peek(BStr::new($input)),
                    Ok((remaining, _)) if remaining == BStr::new($remaining)
                ),
                "assertion failed: parser did not leave expected remaining input {:?}: {:?}",
                $remaining,
                $parser.parse_peek(BStr::new($input)),
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
        assert_backtrack!(parse_authority_form_only, b"");
        assert_partial_incomplete!(parse_authority_form_only, b"", Needed::Unknown);

        assert_backtrack!(parse_authority_form_only, b"localhost");
        assert_backtrack!(parse_authority_form_only, b"user@localhost:3000");
        assert_backtrack!(parse_authority_form_only, b"[::1]");

        assert_ok_remaining!(parse_authority_form_only, b"localhost:3000", b"");
        assert_ok_remaining!(parse_authority_form_only, b"127.0.0.1:80", b"");
        assert_ok_remaining!(parse_authority_form_only, b"[::1]:443", b"");

        let indices = parse_authority_form
            .parse(LocatingSlice::new(&b"localhost:3000"[..]))
            .unwrap();
        assert_eq!(indices.authority, 0..14);
        assert_eq!(indices.host, 0..9);
        assert_eq!(indices.port, 10..14);
    }

    #[test]
    fn parses_absolute_form() {
        assert_backtrack!(parse_absolute_form_only, b"");
        assert_backtrack!(parse_absolute_form_only, b"/foo");
        assert_backtrack!(parse_absolute_form_only, b"htt:p//host");
        assert_partial_incomplete!(parse_absolute_form_only, b"", Needed::new(1));

        assert_ok_remaining!(
            parse_absolute_form_only,
            b"http://127.0.0.1:61761/chunks",
            b""
        );
        assert_ok_remaining!(parse_absolute_form_only, b"https://127.0.0.1:61761", b"");
        assert_ok_remaining!(parse_absolute_form_only, b"http://127.0.0.1?foo=bar", b"");
        assert_ok_remaining!(
            parse_absolute_form_only,
            b"git+http://example.com/repo",
            b""
        );

        let indices = parse_absolute_form
            .parse(LocatingSlice::new(&b"http://127.0.0.1:61761/chunks"[..]))
            .unwrap();
        assert_eq!(indices.scheme, 0..4);
        assert_eq!(indices.authority, 7..22);
        assert_eq!(indices.host, 7..16);
        assert_eq!(indices.port, Some(17..22));
        assert_eq!(indices.path, 22..29);
        assert_eq!(indices.search, None);
    }

    #[test]
    fn parses_asterisk() {
        assert_backtrack!(parse_asterisk, b"");
        assert_partial_incomplete!(parse_asterisk, b"", Needed::Unknown);

        assert_ok_remaining!(parse_asterisk, b"*", b"");
        assert_ok_remaining!(parse_asterisk, b"**", b"*");
    }
}
