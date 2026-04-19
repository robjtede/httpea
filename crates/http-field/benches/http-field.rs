#![allow(missing_docs)]

use std::hint::black_box;

use divan::AllocProfiler;
use http_field::{Field, FieldName, FieldValue};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

#[divan::bench(sample_size = 10_000_000)]
fn field_name_from_slice() {
    black_box(FieldName::from_slice(black_box(b"content-type")));
}

#[divan::bench(sample_size = 10_000_000)]
fn field_name_as_slice() {
    let field = black_box(FieldName::from_slice(b"content-type"));
    black_box(field.as_slice());
}

#[divan::bench(sample_size = 10_000_000)]
fn field_value_from_slice() {
    black_box(FieldValue::from_slice(black_box(
        b"text/plain; charset=utf-8",
    )));
}

#[divan::bench(sample_size = 10_000_000)]
fn field_value_as_slice() {
    let field = black_box(FieldValue::from_slice(b"text/plain; charset=utf-8"));
    black_box(field.as_slice());
}

#[divan::bench(sample_size = 1_000_000)]
fn parse_field_line() {
    black_box(Field::try_from_slice(black_box(
        b"content-type: text/plain; charset=utf-8",
    )))
    .unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn parse_empty_field_value() {
    black_box(Field::try_from_slice(black_box(b"x-empty:\t "))).unwrap();
}

fn main() {
    divan::main();
}
