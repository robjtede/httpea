#![allow(missing_docs)]

use std::hint::black_box;

use divan::AllocProfiler;
use http_chunked::{
    parse_bws, parse_chunk, parse_chunk_data, parse_chunk_ext, parse_chunk_ext_param,
    parse_chunk_ext_val, parse_chunk_header, parse_chunk_size, parse_last_chunk, parse_quoted_pair,
    parse_quoted_string, parse_token,
};
use winnow::Parser;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

#[divan::bench(sample_size = 1_000_000)]
fn chunk_size_short() {
    black_box(parse_chunk_size.parse(black_box(&b"4"[..]))).unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn chunk_size_long() {
    black_box(parse_chunk_size.parse(black_box(&b"000a"[..]))).unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn chunk_ext_empty() {
    black_box(parse_chunk_ext.parse(black_box(&b""[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn chunk_ext_param_token() {
    black_box(parse_chunk_ext_param.parse(black_box(&b";foo=bar"[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn chunk_ext_param_quoted() {
    black_box(parse_chunk_ext_param.parse(black_box(&b";sig=\"abc123xyz\""[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn chunk_ext_multi() {
    black_box(parse_chunk_ext.parse(black_box(&b";foo=bar; baz = \"qux\"; trace=abc123"[..])))
        .unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn chunk_ext_val_token() {
    black_box(parse_chunk_ext_val.parse(black_box(&b"abc123"[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn chunk_ext_val_quoted() {
    black_box(parse_chunk_ext_val.parse(black_box(&b"\"qux\\\\value\""[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn token() {
    black_box(parse_token.parse(black_box(&b"chunk-signature"[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn quoted_string() {
    black_box(parse_quoted_string.parse(black_box(&b"\"sig\\\\value\""[..]))).unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn quoted_pair() {
    black_box(parse_quoted_pair.parse(black_box(&b"\\\""[..]))).unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn bws() {
    black_box(parse_bws.parse(black_box(&b" \t"[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn chunk_header() {
    black_box(parse_chunk_header.parse(black_box(&b"000a;foo=bar\r\n"[..]))).unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn chunk_data() {
    black_box(parse_chunk_data(10).parse(black_box(&b"0123456789"[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn chunk() {
    black_box(parse_chunk.parse(black_box(&b"000a;foo=bar\r\n0123456789\r\n"[..]))).unwrap();
}

#[divan::bench(sample_size = 100_000)]
fn last_chunk() {
    black_box(parse_last_chunk.parse(black_box(&b"0;sig=ok\r\n"[..]))).unwrap();
}

fn main() {
    divan::main();
}
