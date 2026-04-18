#![allow(missing_docs)]

use std::hint::black_box;

use http_method::{GET, Method, POST, QUERY};

#[divan::bench(sample_size = 1_000_000)]
fn parse_get() {
    black_box(Method::try_from_slice(black_box(b"GET")).unwrap());
}

#[divan::bench(sample_size = 1_000_000)]
fn parse_post() {
    black_box(Method::try_from_slice(black_box(b"POST")).unwrap());
}

#[divan::bench(sample_size = 100_000)]
fn parse_extension_method() {
    black_box(Method::try_from_slice(black_box(b"PROPPATCH")).unwrap());
}

#[divan::bench(sample_size = 1_000_000)]
fn parse_query() {
    black_box(Method::try_from_slice(black_box(b"QUERY")).unwrap());
}

#[divan::bench(sample_size = 1_000_000)]
fn compare_equal() {
    black_box(black_box(GET) == black_box(GET));
}

#[divan::bench(sample_size = 1_000_000)]
fn compare_not_equal() {
    black_box(black_box(POST) == black_box(QUERY));
}

fn main() {
    divan::main();
}
