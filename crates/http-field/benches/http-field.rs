#![allow(missing_docs)]

use std::hint::black_box;

use divan::AllocProfiler;
use http_field::{Field, FieldName, FieldValue};

#[allow(dead_code, unused_imports, unused_macros)]
#[path = "../src/parsing.rs"]
mod parsing;

const LONG_FIELD_LINE: &[u8] =
    b"cache-control: public, max-age=31536000, stale-while-revalidate=60, stale-if-error=86400";
const LONG_FIELD_LINE_WITH_TRAILING_OWS: &[u8] =
    b"cache-control: public, max-age=31536000, stale-while-revalidate=60, stale-if-error=86400                                \t\t\t\t";
const VERY_LONG_FIELD_LINE_WITH_TRAILING_OWS: &[u8] = b"set-cookie: session_id=0123456789abcdef0123456789abcdef; Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800; Priority=High                                                                \t\t\t\t\t\t\t\t";

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

#[divan::bench(sample_size = 10_000_000)]
fn field_name_try_from_slice() {
    black_box(FieldName::try_from_slice(black_box(b"content-type"))).unwrap();
}

#[divan::bench(sample_size = 10_000_000)]
fn field_name_as_slice() {
    let field = black_box(FieldName::try_from_slice(b"content-type").unwrap());
    black_box(field.as_slice());
}

#[divan::bench(sample_size = 10_000_000)]
fn field_value_try_from_slice() {
    black_box(FieldValue::try_from_slice(black_box(
        b"text/plain; charset=utf-8",
    )))
    .unwrap();
}

#[divan::bench(sample_size = 10_000_000)]
fn field_value_as_slice() {
    let field = black_box(FieldValue::try_from_slice(b"text/plain; charset=utf-8").unwrap());
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

#[divan::bench(sample_size = 1_000_000)]
fn direct_parse_field_line() {
    black_box(parsing::parse_field(black_box(
        b"content-type: text/plain; charset=utf-8",
    )))
    .unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn direct_parse_empty_field_value() {
    black_box(parsing::parse_field(black_box(b"x-empty:\t "))).unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn direct_parse_long_field_line() {
    black_box(parsing::parse_field(black_box(LONG_FIELD_LINE))).unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn direct_parse_long_field_line_with_trailing_ows() {
    black_box(parsing::parse_field(black_box(
        LONG_FIELD_LINE_WITH_TRAILING_OWS,
    )))
    .unwrap();
}

#[divan::bench(sample_size = 1_000_000)]
fn direct_parse_very_long_field_line_with_trailing_ows() {
    black_box(parsing::parse_field(black_box(
        VERY_LONG_FIELD_LINE_WITH_TRAILING_OWS,
    )))
    .unwrap();
}

fn main() {
    divan::main();
}
