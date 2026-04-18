#![allow(missing_docs)]

use std::hint::black_box;

use http_version::Version;

#[divan::bench(sample_size = 10_000)]
fn clone_http_1_1() {
    black_box(black_box(Version::Http1_1).clone());
}

#[divan::bench(sample_size = 10_000)]
fn clone_http_2() {
    black_box(black_box(Version::Http2).clone());
}

#[divan::bench(sample_size = 10_000)]
fn clone_http_3() {
    black_box(black_box(Version::Http3).clone());
}

fn main() {
    divan::main();
}
