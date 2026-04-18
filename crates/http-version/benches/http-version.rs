#![allow(missing_docs)]

use std::hint::black_box;

use http_version::Version;

#[divan::bench(sample_size = 1_000_000)]
fn parse_http_1_1() {
    black_box(Version::try_from_slice(black_box(b"HTTP/1.1")).unwrap());
}

#[divan::bench(sample_size = 1_000_000)]
fn parse_http_2() {
    black_box(Version::try_from_slice(black_box(b"HTTP/2")).unwrap());
}

#[divan::bench(sample_size = 1_000_000)]
fn parse_http_3() {
    black_box(Version::try_from_slice(black_box(b"HTTP/3")).unwrap());
}

#[divan::bench(sample_size = 1_000_000)]
fn as_slice_http_1_1() {
    black_box(black_box(Version::Http1_1).as_slice());
}

fn main() {
    divan::main();
}
