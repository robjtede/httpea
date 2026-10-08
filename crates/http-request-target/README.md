# `http-request-target`

<!-- prettier-ignore-start -->

[![crates.io](https://img.shields.io/crates/v/http-request-target?label=latest)](https://crates.io/crates/http-request-target)
[![Documentation](https://docs.rs/http-request-target/badge.svg?version=0.1.0)](https://docs.rs/http-request-target/0.1.0)
![MIT licensed](https://img.shields.io/crates/l/http-request-target.svg)
<br />
[![dependency status](https://deps.rs/crate/http-request-target/0.1.0/status.svg)](https://deps.rs/crate/http-request-target/0.1.0)
[![Download](https://img.shields.io/crates/d/http-request-target.svg)](https://crates.io/crates/http-request-target)
[![CI](https://github.com/robjtede/httpea/actions/workflows/ci.yml/badge.svg)](https://github.com/robjtede/httpea/actions/workflows/ci.yml)

<!-- prettier-ignore-end -->

<!-- cargo-rdme start -->

HTTP/1.1 request-target parser from
[RFC 9112](https://datatracker.ietf.org/doc/html/rfc9112).

<!-- cargo-rdme end -->

HTTP/1.1 request-target parser with a zero-copy bias.

## Absolute-Form Policy

RFC 9112 defines:

```text
absolute-form = absolute-URI
```

This crate intentionally narrows that production for implementation purposes to the
authority-based URI shape commonly used by HTTP-family request targets:

```text
scheme "://" authority path-abempty [ "?" query ]
```

This means:

- `http://example.com/path` is accepted
- `https://example.com` is accepted
- `git+http://example.com/repo` is accepted
- `htt:p//host` is rejected

The intent is to parse the request-target portion of an HTTP request line, not to act as a
generic RFC 3986 absolute-URI parser. In practice that means this crate accepts absolute-form
targets that carry an authority and rejects other generic absolute-URI shapes even if they are
permitted by the broader URI grammar.
