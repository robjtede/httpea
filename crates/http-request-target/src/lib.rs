//! HTTP/1.1 request-target (RFC 9112) parser.

// #![no_std]

extern crate alloc;

use alloc::string::String;
use core::str;

mod error;

use crate::error::ParseRequestTarget;

/// See <https://datatracker.ietf.org/doc/html/rfc9112#name-request-target>.
#[derive(Debug, Clone)]
pub enum RequestTarget {
    /// Origin form.
    Origin(String),

    /// ```plain
    /// absolute-form = absolute-URI
    /// GET http://www.example.org/pub/WWW/TheProject.html HTTP/1.1
    /// ```
    Absolute(String),

    /// ```plain
    /// authority-form = uri-host ":" port
    /// CONNECT www.example.com:80 HTTP/1.1
    /// ```
    Authority(String),

    /// Asterisk form.
    ///
    /// ```plain
    /// asterisk-form = "*"
    /// OPTIONS * HTTP/1.1
    /// ```
    Asterisk,
}

impl RequestTarget {
    /// Parse request target from slice.
    pub fn try_from_slice(slice: &[u8]) -> Result<Self, ParseRequestTarget> {
        if slice.is_empty() {
            return Err(ParseRequestTarget);
        }

        if slice == b"*" {
            return Ok(Self::Asterisk);
        }

        let mut buf = String::new();

        match () {
            _ if memchr::memchr(b'/', slice).is_none() => {
                let authority_form = parse_authority_form(slice, &mut buf)?;
                Ok(Self::Authority(authority_form))
            }

            _ => {
                parse_origin_form(slice, &mut buf)?;

                Ok(Self::Origin(buf))
            }
        }
    }
}

/// ```plain
/// GET /where?q=now HTTP/1.1
///
/// origin-form   = absolute-path [ "?" query ]
/// absolute-path = 1*( "/" segment )
/// segment       = *pchar
/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
/// unreserved    = ALPHA / DIGIT / "-" / "." / "_" / "~"
/// pct-encoded   = "%" HEXDIG HEXDIG
/// sub-delims    = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="
/// query         = *( pchar / "/" / "?" )
/// ```
fn parse_origin_form(slice: &[u8], buf: &mut String) -> Result<(), ParseRequestTarget> {
    match memchr::memchr(b'?', slice) {
        // has query
        Some(query_start) => {
            parse_path(&slice[..query_start], buf)?;
            buf.push('?');
            parse_query(&slice[query_start + 1..], buf)?;
        }

        // just a path
        None => {
            parse_path(slice, buf)?;
        }
    };

    Ok(())
}

fn parse_path(mut slice: &[u8], buf: &mut String) -> Result<(), ParseRequestTarget> {
    if slice.is_empty() {
        return Err(ParseRequestTarget);
    }

    while !slice.is_empty() {
        if !slice.starts_with(b"/") {
            return Err(ParseRequestTarget);
        }

        buf.push('/');
        let seg_len = parse_segment(&slice[1..], buf)?;

        // advance by (segment delimiter + contents)
        slice = &slice[1 + seg_len..];
    }

    Ok(())
}

fn parse_segment(slice: &[u8], buf: &mut String) -> Result<usize, ParseRequestTarget> {
    if slice.is_empty() {
        return Ok(0);
    }

    let mut iter = slice.iter().peekable();

    let mut i = 0;

    loop {
        let Some(b) = iter.next_if(|&&b| b != b'/') else {
            // stop consuming if next char is start-of-next-segment or EOL
            break;
        };

        let consumed = match b {
            // unreserved
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'.' | b'_' | b'~' => 1,

            // pct-encoded
            b'%' => {
                // HEXDIGIT
                match iter.next() {
                    Some(b'0'..=b'9' | b'A'..=b'F') => {}
                    Some(&_) => return Err(ParseRequestTarget),
                    None => return Err(ParseRequestTarget),
                }

                // HEXDIGIT
                match iter.next() {
                    Some(b'0'..=b'9' | b'A'..=b'F') => {}
                    Some(&_) => return Err(ParseRequestTarget),
                    None => return Err(ParseRequestTarget),
                }

                3
            }

            // sub-delims
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' => 1,

            _ => return Err(ParseRequestTarget),
        };

        i += consumed;
    }

    let segment = str::from_utf8(&slice[..i]).map_err(|_err| ParseRequestTarget)?;
    buf.push_str(segment);

    Ok(i)
}

fn parse_query(slice: &[u8], buf: &mut String) -> Result<(), ParseRequestTarget> {
    let mut iter = slice.iter().peekable();

    loop {
        let Some(b) = iter.next_if(|&&b| b != b'#') else {
            // stop consuming if next char is start-of-fragment or EOL
            break;
        };

        match b {
            // unreserved
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'.' | b'_' | b'~' => {}

            // pct-encoded
            b'%' => {
                // HEXDIGIT
                match iter.next() {
                    Some(b'0'..=b'9' | b'A'..=b'F') => {}
                    Some(&_) => return Err(ParseRequestTarget),
                    None => return Err(ParseRequestTarget),
                }

                // HEXDIGIT
                match iter.next() {
                    Some(b'0'..=b'9' | b'A'..=b'F') => {}
                    Some(&_) => return Err(ParseRequestTarget),
                    None => return Err(ParseRequestTarget),
                }
            }

            // sub-delims
            b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' => {}

            // additional chars allowed in query
            b'/' | b'?' => {}

            _ => return Err(ParseRequestTarget),
        }
    }

    let query = str::from_utf8(slice).map_err(|_err| ParseRequestTarget)?;
    buf.push_str(query);

    Ok(())
}

fn parse_authority_form(_slice: &[u8], _buf: &mut String) -> Result<String, ParseRequestTarget> {
    todo!()
}
