# `http-request-target`

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
