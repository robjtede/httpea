#![allow(missing_docs)]

use std::hint::black_box;

use http_status_code::StatusCode;

#[divan::bench(sample_size = 10_000)]
fn from_u16_ok() {
    black_box(StatusCode::from_u16(black_box(200)));
}

#[divan::bench(sample_size = 10_000)]
fn from_u16_max() {
    black_box(StatusCode::from_u16(black_box(999)));
}

#[divan::bench(sample_size = 10_000)]
fn as_text_bytes() {
    let status = StatusCode::from_u16(200);
    black_box(status.as_text_bytes());
}

fn main() {
    divan::main();
}
