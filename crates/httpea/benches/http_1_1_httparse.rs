#![allow(missing_docs)]

use std::{hint::black_box, mem::MaybeUninit};

use divan::AllocProfiler;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

const REQUEST_HEAD_MINIMAL: &[u8] = b"GET / HTTP/1.1\r\n\r\n";
const REQUEST_HEAD_WITH_HEADERS: &[u8] =
    b"GET /where?q=now HTTP/1.1\r\nhost: example.com\r\naccept: application/json\r\n\r\n";
const REQUEST_HEAD_WITH_BUFFERED_BODY: &[u8] =
    b"POST /upload HTTP/1.1\r\nhost: example.com\r\ncontent-length: 4\r\n\r\nbody";
const RESPONSE_HEAD_WITH_HEADERS: &[u8] =
    b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nserver: httpea\r\n\r\nbody";
const GENERIC_HEAD_RESPONSE: &[u8] = b"HTTP/1.1 503 Service Unavailable\r\nretry-after: 60\r\n\r\n";
const TRAILER_SECTION_EMPTY: &[u8] = b"\r\n";
const TRAILER_SECTION_WITH_FIELDS: &[u8] = b"etag: abc\r\nexpires: now\r\nx-checksum: 1234\r\n\r\n";
const TRAILER_FIELD_AS_SECTION: &[u8] = b"etag: abc\r\n\r\n";

#[divan::bench(sample_size = 100_000)]
fn parse_request_head_minimal() {
    let mut headers = [];
    let mut uninit_headers: [MaybeUninit<::httparse::Header<'_>>; 0] = [];
    let mut request = ::httparse::Request::new(&mut headers);
    let config = ::httparse::ParserConfig::default();

    black_box(config.parse_request_with_uninit_headers(
        black_box(&mut request),
        black_box(REQUEST_HEAD_MINIMAL),
        black_box(&mut uninit_headers),
    ))
    .unwrap()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_request_head_with_headers() {
    let mut headers = [];
    let mut uninit_headers: [MaybeUninit<::httparse::Header<'_>>; 8] =
        [const { MaybeUninit::uninit() }; 8];
    let mut request = ::httparse::Request::new(&mut headers);
    let config = ::httparse::ParserConfig::default();

    black_box(config.parse_request_with_uninit_headers(
        black_box(&mut request),
        black_box(REQUEST_HEAD_WITH_HEADERS),
        black_box(&mut uninit_headers),
    ))
    .unwrap()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_request_head_with_buffered_body() {
    let mut headers = [];
    let mut uninit_headers: [MaybeUninit<::httparse::Header<'_>>; 8] =
        [const { MaybeUninit::uninit() }; 8];
    let mut request = ::httparse::Request::new(&mut headers);
    let config = ::httparse::ParserConfig::default();

    black_box(config.parse_request_with_uninit_headers(
        black_box(&mut request),
        black_box(REQUEST_HEAD_WITH_BUFFERED_BODY),
        black_box(&mut uninit_headers),
    ))
    .unwrap()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_response_head_with_headers() {
    let mut headers = [];
    let mut uninit_headers: [MaybeUninit<::httparse::Header<'_>>; 8] =
        [const { MaybeUninit::uninit() }; 8];
    let mut response = ::httparse::Response::new(&mut headers);
    let config = ::httparse::ParserConfig::default();

    black_box(config.parse_response_with_uninit_headers(
        black_box(&mut response),
        black_box(RESPONSE_HEAD_WITH_HEADERS),
        black_box(&mut uninit_headers),
    ))
    .unwrap()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_generic_head_response() {
    let mut headers = [];
    let mut uninit_headers: [MaybeUninit<::httparse::Header<'_>>; 8] =
        [const { MaybeUninit::uninit() }; 8];
    let mut response = ::httparse::Response::new(&mut headers);
    let config = ::httparse::ParserConfig::default();

    black_box(config.parse_response_with_uninit_headers(
        black_box(&mut response),
        black_box(GENERIC_HEAD_RESPONSE),
        black_box(&mut uninit_headers),
    ))
    .unwrap()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_trailer_section_empty() {
    let mut headers = [::httparse::EMPTY_HEADER; 1];

    black_box(::httparse::parse_headers(
        black_box(TRAILER_SECTION_EMPTY),
        black_box(&mut headers),
    ))
    .unwrap()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_trailer_section_with_fields() {
    let mut headers = [::httparse::EMPTY_HEADER; 8];

    black_box(::httparse::parse_headers(
        black_box(TRAILER_SECTION_WITH_FIELDS),
        black_box(&mut headers),
    ))
    .unwrap()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_trailer_field_line() {
    let mut headers = [::httparse::EMPTY_HEADER; 4];

    black_box(::httparse::parse_headers(
        black_box(TRAILER_FIELD_AS_SECTION),
        black_box(&mut headers),
    ))
    .unwrap()
    .unwrap();
}

fn main() {
    divan::main();
}
