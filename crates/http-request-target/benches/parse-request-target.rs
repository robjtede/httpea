#![allow(missing_docs)]

use std::hint::black_box;

use divan::AllocProfiler;
use http_request_target::RequestTarget;

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

#[divan::bench(sample_count = 1_000, sample_size = 10_000)]
fn origin_root() {
    black_box(RequestTarget::try_from_slice(black_box(b"/")).unwrap());
}

#[divan::bench(sample_count = 1_000, sample_size = 10_000)]
fn origin_path() {
    black_box(RequestTarget::try_from_slice(black_box(b"/where")).unwrap());
}

#[divan::bench(sample_count = 1_000, sample_size = 10_000)]
fn origin_query() {
    black_box(RequestTarget::try_from_slice(black_box(b"/where?q=now")).unwrap());
}

#[divan::bench(sample_count = 5_000, sample_size = 10_000)]
fn asterisk() {
    black_box(RequestTarget::try_from_slice(black_box(b"*")).unwrap());
}

#[divan::bench(sample_count = 1_000, sample_size = 10_000)]
fn authority_reg_name() {
    black_box(RequestTarget::try_from_slice(black_box(b"localhost:3000")).unwrap());
}

#[divan::bench(sample_count = 1_000, sample_size = 10_000)]
fn authority_ipv4() {
    black_box(RequestTarget::try_from_slice(black_box(b"127.0.0.1:3000")).unwrap());
}

#[divan::bench(sample_count = 1_000, sample_size = 10_000)]
fn authority_ipv6() {
    black_box(RequestTarget::try_from_slice(black_box(b"[::1]:3000")).unwrap());
}

fn main() {
    divan::main();
}
