#![allow(missing_docs)]

use std::hint::black_box;

use http_field::{FieldName, FieldValue};

#[divan::bench(sample_size = 10_000)]
fn field_name_from_slice() {
    black_box(FieldName::from_slice(black_box(b"content-type")));
}

#[divan::bench(sample_size = 10_000)]
fn field_name_as_slice() {
    let field = FieldName::from_slice(b"content-type");
    black_box(field.as_slice());
}

#[divan::bench(sample_size = 10_000)]
fn field_value_from_slice() {
    black_box(FieldValue::from_slice(black_box(
        b"text/plain; charset=utf-8",
    )));
}

#[divan::bench(sample_size = 10_000)]
fn field_value_as_slice() {
    let field = FieldValue::from_slice(b"text/plain; charset=utf-8");
    black_box(field.as_slice());
}

fn main() {
    divan::main();
}
