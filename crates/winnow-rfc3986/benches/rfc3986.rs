#![allow(missing_docs)]

use std::hint::black_box;

use divan::AllocProfiler;
use winnow::Parser as _;
use winnow_rfc3986::{
    parse_absolute_uri, parse_authority, parse_ip_literal, parse_path_absolute, parse_relative_ref,
    parse_uri,
};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

#[divan::bench(sample_size = 100_000)]
fn absolute_uri_http() {
    black_box(
        parse_absolute_uri
            .parse(black_box(&b"https://example.com:8443/a/b/c?q=1&lang=en"[..]))
            .unwrap(),
    );
}

#[divan::bench(sample_size = 100_000)]
fn absolute_uri_mailto() {
    black_box(
        parse_absolute_uri
            .parse(black_box(&b"mailto:John.Doe@example.com?subject=Hi"[..]))
            .unwrap(),
    );
}

#[divan::bench(sample_size = 100_000)]
fn relative_ref_nested() {
    black_box(
        parse_relative_ref
            .parse(black_box(&b"../images/icons/logo.svg?v=20260422#hero"[..]))
            .unwrap(),
    );
}

#[divan::bench(sample_size = 100_000)]
fn full_uri_with_fragment() {
    black_box(
        parse_uri
            .parse(black_box(&b"https://user:pass@example.com:443/docs/latest?q=1#install"[..]))
            .unwrap(),
    );
}

#[divan::bench(sample_size = 100_000)]
fn authority_ipv6() {
    black_box(
        parse_authority
            .parse(black_box(&b"user:pass@[2001:db8::1]:443"[..]))
            .unwrap(),
    );
}

#[divan::bench(sample_size = 100_000)]
fn ip_literal_ipvfuture() {
    black_box(
        parse_ip_literal
            .parse(black_box(&b"[vF.future-token:part-1]"[..]))
            .unwrap(),
    );
}

#[divan::bench(sample_size = 100_000)]
fn path_absolute_deep() {
    black_box(
        parse_path_absolute
            .parse(black_box(&b"/a/b/c/d/e/f/g%20h/i;j=k"[..]))
            .unwrap(),
    );
}

fn main() {
    divan::main();
}
