# `http-chunked`

<!-- cargo-rdme start -->

HTTP/1.1 chunked transfer coding parsers.

This crate exposes small composable parser functions for the chunk syntax from
[RFC 9112 §7.1] and [RFC 9112 §7.1.1].
It intentionally does not parse the optional `trailer-section`.

[RFC 9112 §7.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
[RFC 9112 §7.1.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1

<!-- cargo-rdme end -->
