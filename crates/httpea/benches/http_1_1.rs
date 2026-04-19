#![allow(missing_docs)]

use std::{hint::black_box, mem::MaybeUninit};

use divan::AllocProfiler;
use http_field::Field;

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
const TRAILER_FIELD_LINE: &[u8] = b"etag: abc\r\n";

#[divan::bench(sample_size = 100_000)]
fn parse_request_head_minimal() {
    let mut fields: [MaybeUninit<Field<'_>>; 0] = [];

    black_box(::httpea::http_1_1::parse_request_head(
        black_box(REQUEST_HEAD_MINIMAL),
        black_box(&mut fields),
    ))
    .unwrap()
    .into_complete()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_request_head_with_headers() {
    let mut fields: [MaybeUninit<Field<'_>>; 8] = [const { MaybeUninit::uninit() }; 8];

    black_box(::httpea::http_1_1::parse_request_head(
        black_box(REQUEST_HEAD_WITH_HEADERS),
        black_box(&mut fields),
    ))
    .unwrap()
    .into_complete()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_request_head_with_buffered_body() {
    let mut fields: [MaybeUninit<Field<'_>>; 8] = [const { MaybeUninit::uninit() }; 8];

    black_box(::httpea::http_1_1::parse_request_head(
        black_box(REQUEST_HEAD_WITH_BUFFERED_BODY),
        black_box(&mut fields),
    ))
    .unwrap()
    .into_complete()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_response_head_with_headers() {
    let mut fields: [MaybeUninit<Field<'_>>; 8] = [const { MaybeUninit::uninit() }; 8];

    black_box(::httpea::http_1_1::parse_response_head(
        black_box(RESPONSE_HEAD_WITH_HEADERS),
        black_box(&mut fields),
    ))
    .unwrap()
    .into_complete()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_generic_head_response() {
    let mut fields: [MaybeUninit<Field<'_>>; 8] = [const { MaybeUninit::uninit() }; 8];

    black_box(::httpea::http_1_1::parse_head(
        black_box(GENERIC_HEAD_RESPONSE),
        black_box(&mut fields),
    ))
    .unwrap()
    .into_complete()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_trailer_section_empty() {
    let mut fields: [MaybeUninit<Field<'_>>; 0] = [];

    black_box(::httpea::http_1_1::parse_trailer_section(
        black_box(TRAILER_SECTION_EMPTY),
        black_box(&mut fields),
    ))
    .unwrap()
    .into_complete()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_trailer_section_with_fields() {
    let mut fields: [MaybeUninit<Field<'_>>; 8] = [const { MaybeUninit::uninit() }; 8];

    black_box(::httpea::http_1_1::parse_trailer_section(
        black_box(TRAILER_SECTION_WITH_FIELDS),
        black_box(&mut fields),
    ))
    .unwrap()
    .into_complete()
    .unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn parse_trailer_field_line() {
    black_box(::httpea::http_1_1::parse_trailer_field(black_box(
        TRAILER_FIELD_LINE,
    )))
    .unwrap()
    .into_complete()
    .unwrap();
}

fn main() {
    divan::main();
}
