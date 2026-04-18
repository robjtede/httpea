#![allow(missing_docs)]

use std::hint::black_box;

use http_method::{GET, Method, POST};

#[divan::bench(sample_size = 10_000)]
fn parse_get() {
    black_box(Method::try_from_slice(black_box(b"GET")).unwrap());
}

#[divan::bench(sample_size = 10_000)]
fn parse_post() {
    black_box(Method::try_from_slice(black_box(b"POST")).unwrap());
}

#[divan::bench(sample_size = 10_000)]
fn parse_extension_method() {
    black_box(Method::try_from_slice(black_box(b"PRI")).unwrap());
}

#[divan::bench(sample_size = 10_000)]
fn compare_not_equal() {
    black_box(black_box(GET) == black_box(POST));
}

fn main() {
    divan::main();
}
