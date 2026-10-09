# `http-chunked`

<!-- prettier-ignore-start -->

[![crates.io](https://img.shields.io/crates/v/http-chunked?label=latest)](https://crates.io/crates/http-chunked)
[![Documentation](https://docs.rs/http-chunked/badge.svg?version=0.0.2)](https://docs.rs/http-chunked/0.0.2)
![MIT licensed](https://img.shields.io/crates/l/http-chunked.svg)
<br />
[![dependency status](https://deps.rs/crate/http-chunked/0.0.2/status.svg)](https://deps.rs/crate/http-chunked/0.0.2)
[![Download](https://img.shields.io/crates/d/http-chunked.svg)](https://crates.io/crates/http-chunked)
[![CI](https://github.com/robjtede/httpea/actions/workflows/ci.yml/badge.svg)](https://github.com/robjtede/httpea/actions/workflows/ci.yml)

<!-- prettier-ignore-end -->

<!-- cargo-rdme start -->

HTTP/1.1 chunked transfer coding parsers.

This crate exposes small composable parser functions for the chunk syntax from
[RFC 9112 §7.1] and [RFC 9112 §7.1.1].
It intentionally does not parse the optional `trailer-section`.

[RFC 9112 §7.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
[RFC 9112 §7.1.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1

<!-- cargo-rdme end -->
