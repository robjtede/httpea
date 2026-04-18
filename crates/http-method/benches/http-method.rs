#![allow(missing_docs)]

use std::hint::black_box;

use http_method::{GET, POST};

#[divan::bench(sample_size = 10_000)]
fn clone_get() {
    black_box(black_box(GET).clone());
}

#[divan::bench(sample_size = 10_000)]
fn clone_post() {
    black_box(black_box(POST).clone());
}

#[divan::bench(sample_size = 10_000)]
fn compare_equal() {
    black_box(black_box(GET) == black_box(GET));
}

#[divan::bench(sample_size = 10_000)]
fn compare_not_equal() {
    black_box(black_box(GET) == black_box(POST));
}

fn main() {
    divan::main();
}
