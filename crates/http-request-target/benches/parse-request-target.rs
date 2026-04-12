#![allow(missing_docs)]

use std::hint::black_box;

use http_request_target::RequestTarget;

#[divan::bench]
fn origin_root() {
    black_box(RequestTarget::try_from_slice(black_box(b"/")).unwrap());
}

#[divan::bench]
fn origin_path() {
    black_box(RequestTarget::try_from_slice(black_box(b"/where")).unwrap());
}

#[divan::bench]
fn origin_query() {
    black_box(RequestTarget::try_from_slice(black_box(b"/where?q=now")).unwrap());
}

#[divan::bench]
fn asterisk() {
    black_box(RequestTarget::try_from_slice(black_box(b"*")).unwrap());
}

fn main() {
    divan::main();
}
