use winnow::{
    combinator::{alt, opt},
    error::ErrMode,
    prelude::*,
    stream::{Compare, Stream, StreamIsPartial},
    token::take_while,
};

/// Parses a complete non-terminal chunk and returns its decoded size in octets.
///
/// # Chunk Examples
///
/// ```plain
/// 4\r\nWiki\r\n
/// 000a;foo=bar\r\n0123456789\r\n
/// ```
///
/// # BNF
///
/// ```plain
/// chunk      = chunk-size [ chunk-ext ] CRLF
///              chunk-data CRLF
/// chunk-size = 1*HEXDIG
/// chunk-data = 1*OCTET
/// ```
///
/// See: [RFC 9112 §7.1]
///
/// [RFC 9112 §7.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
pub fn parse_chunk<I>(input: &mut I) -> ModalResult<usize>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    let checkpoint = input.checkpoint();
    let size = parse_chunk_header.parse_next(input)?;

    if size == 0 {
        input.reset(&checkpoint);
        return Err(ErrMode::from_input(&*input));
    }

    parse_chunk_data(size).parse_next(input)?;
    parse_crlf(input)?;

    Ok(size)
}

/// Parses a `chunk-size [ chunk-ext ] CRLF` header and returns the decoded chunk size in octets.
///
/// # Chunk Examples
///
/// ```plain
/// 4\r\n
/// 4;foo=bar\r\n
/// 0;sig=ok\r\n
/// ```
///
/// # BNF
///
/// ```plain
/// chunk-size = 1*HEXDIG
/// chunk-ext  = *( BWS ";" BWS chunk-ext-name
///                [ BWS "=" BWS chunk-ext-val ] )
/// ```
///
/// See:
/// - [RFC 9112 §7.1]
/// - [RFC 9112 §7.1.1]
///
/// [RFC 9112 §7.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
/// [RFC 9112 §7.1.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1
pub fn parse_chunk_header<I>(input: &mut I) -> ModalResult<usize>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    let size = parse_chunk_size.parse_next(input)?;
    parse_chunk_ext.parse_next(input)?;
    parse_crlf(input)?;

    Ok(size)
}

/// Parses `chunk-size` and returns its decoded numeric value.
///
/// # Chunk Examples
///
/// ```plain
/// 4
/// a
/// 000a
/// ```
///
/// # BNF
///
/// ```plain
/// chunk-size = 1*HEXDIG
/// ```
///
/// See: [RFC 9112 §7.1]
///
/// [RFC 9112 §7.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
pub fn parse_chunk_size<I>(input: &mut I) -> ModalResult<usize>
where
    I: Stream<Token = u8> + StreamIsPartial,
    I::Slice: AsRef<[u8]>,
{
    parse_chunk_size_inner(input).map(|(size, _)| size)
}

/// Parses exactly `size` octets of `chunk-data`.
///
/// This parser does not consume the trailing chunk CRLF.
///
/// # Chunk Examples
///
/// ```plain
/// Wiki
/// 0123456789
/// ```
///
/// # BNF
///
/// ```plain
/// chunk-data = 1*OCTET
/// ```
///
/// See: [RFC 9112 §7.1]
///
/// [RFC 9112 §7.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
pub fn parse_chunk_data<I>(size: usize) -> impl Parser<I, (), ErrMode<winnow::error::ContextError>>
where
    I: Stream<Token = u8> + StreamIsPartial,
    I::Slice: AsRef<[u8]>,
{
    move |input: &mut I| {
        if size == 0 {
            return Err(ErrMode::from_input(&*input));
        }

        let remaining = input.peek_slice(input.eof_offset());

        if remaining.as_ref().len() < size {
            return Err(ErrMode::from_input(&*input));
        }

        let _ = input.next_slice(size);
        Ok(())
    }
}

/// Parses `last-chunk`.
///
/// # Chunk Examples
///
/// ```plain
/// 0\r\n
/// 000;sig=ok\r\n
/// ```
///
/// # BNF
///
/// ```plain
/// last-chunk = 1*("0") [ chunk-ext ] CRLF
/// ```
///
/// See: [RFC 9112 §7.1]
///
/// [RFC 9112 §7.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1
pub fn parse_last_chunk<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    let checkpoint = input.checkpoint();
    let (_, is_last) = parse_chunk_size_inner(input)?;

    if !is_last {
        input.reset(&checkpoint);
        return Err(ErrMode::from_input(&*input));
    }

    parse_chunk_ext.parse_next(input)?;
    parse_crlf(input)
}

/// Parses `chunk-ext`.
///
/// # Chunk Examples
///
/// ```plain
/// <empty>
/// ;foo
/// ;foo=bar; baz = "qux"
/// ```
///
/// # BNF
///
/// ```plain
/// chunk-ext      = *( BWS ";" BWS chunk-ext-name
///                     [ BWS "=" BWS chunk-ext-val ] )
/// chunk-ext-name = token
/// chunk-ext-val  = token / quoted-string
/// ```
///
/// See:
/// - [RFC 9112 §7.1.1]
/// - [RFC 9110 §5.6.2]
/// - [RFC 9110 §5.6.4]
///
/// [RFC 9112 §7.1.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1
/// [RFC 9110 §5.6.2]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.2
/// [RFC 9110 §5.6.4]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4
pub fn parse_chunk_ext<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    while has_chunk_ext_prefix(input.peek_slice(input.eof_offset()).as_ref()) {
        parse_chunk_ext_param.parse_next(input)?;
    }

    Ok(())
}

/// Parses one `;`-prefixed chunk extension parameter.
///
/// # Chunk Examples
///
/// ```plain
/// ;foo
/// ;foo=bar
/// ;sig="abc123"
/// ```
///
/// # BNF
///
/// ```plain
/// ";" BWS chunk-ext-name [ BWS "=" BWS chunk-ext-val ]
/// ```
///
/// See:
/// - [RFC 9112 §7.1.1]
/// - [RFC 9110 §5.6.2]
/// - [RFC 9110 §5.6.4]
///
/// [RFC 9112 §7.1.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1
/// [RFC 9110 §5.6.2]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.2
/// [RFC 9110 §5.6.4]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4
pub fn parse_chunk_ext_param<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    parse_bws.parse_next(input)?;
    b';'.parse_next(input)?;
    parse_bws.parse_next(input)?;
    parse_token.parse_next(input)?;
    opt((parse_bws, b'=', parse_bws, parse_chunk_ext_val)).parse_next(input)?;

    Ok(())
}

/// Parses `chunk-ext-val`.
///
/// # Chunk Examples
///
/// ```plain
/// foo
/// bar
/// "qux"
/// ```
///
/// # BNF
///
/// ```plain
/// chunk-ext-val = token / quoted-string
/// ```
///
/// See:
/// - [RFC 9112 §7.1.1]
/// - [RFC 9110 §5.6.2]
/// - [RFC 9110 §5.6.4]
///
/// [RFC 9112 §7.1.1]: https://datatracker.ietf.org/doc/html/rfc9112#section-7.1.1
/// [RFC 9110 §5.6.2]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.2
/// [RFC 9110 §5.6.4]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4
pub fn parse_chunk_ext_val<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    alt((parse_token, parse_quoted_string)).parse_next(input)
}

/// Parses `token`.
///
/// # Chunk Examples
///
/// ```plain
/// foo
/// chunk-signature
/// abc123
/// ```
///
/// # BNF
///
/// ```plain
/// token = 1*tchar
/// ```
///
/// See: [RFC 9110 §5.6.2]
///
/// [RFC 9110 §5.6.2]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.2
pub fn parse_token<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    take_while(1.., is_tchar).void().parse_next(input)
}

/// Parses `quoted-string`.
///
/// # Chunk Examples
///
/// ```plain
/// "qux"
/// "sig\\value"
/// ```
///
/// # BNF
///
/// ```plain
/// quoted-string = DQUOTE *( qdtext / quoted-pair ) DQUOTE
/// ```
///
/// See: [RFC 9110 §5.6.4]
///
/// [RFC 9110 §5.6.4]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4
pub fn parse_quoted_string<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
    I::Slice: AsRef<[u8]>,
{
    b'"'.parse_next(input)?;

    loop {
        let remaining = input.peek_slice(input.eof_offset());
        let remaining = remaining.as_ref();

        match remaining.first().copied() {
            Some(b'"') => return b'"'.parse_next(input).map(|_| ()),
            Some(b'\\') => parse_quoted_pair.parse_next(input)?,
            Some(byte) if is_qdtext_byte(byte) => {
                let len = remaining
                    .iter()
                    .take_while(|&&byte| is_qdtext_byte(byte))
                    .count();
                let _ = input.next_slice(len);
            }
            _ => return Err(ErrMode::from_input(&*input)),
        }
    }
}

/// Parses `quoted-pair`.
///
/// # Chunk Examples
///
/// ```plain
/// \"
/// \\
/// ```
///
/// # BNF
///
/// ```plain
/// quoted-pair = "\" ( HTAB / SP / VCHAR / obs-text )
/// ```
///
/// See: [RFC 9110 §5.6.4]
///
/// [RFC 9110 §5.6.4]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4
pub fn parse_quoted_pair<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
{
    b'\\'.parse_next(input)?;
    take_while(1..=1, is_quoted_pair_byte)
        .void()
        .parse_next(input)
}

/// Parses optional bad whitespace.
///
/// # Chunk Examples
///
/// ```plain
/// <empty>
/// " "
/// "\t"
/// ```
///
/// # BNF
///
/// ```plain
/// BWS = OWS
/// OWS = *( SP / HTAB )
/// ```
///
/// See: [RFC 9110 §5.6.3]
///
/// [RFC 9110 §5.6.3]: https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.3
pub fn parse_bws<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial,
{
    take_while(.., is_ows_byte).void().parse_next(input)
}

fn parse_chunk_size_inner<I>(input: &mut I) -> ModalResult<(usize, bool)>
where
    I: Stream<Token = u8> + StreamIsPartial,
    I::Slice: AsRef<[u8]>,
{
    let remaining = input.peek_slice(input.eof_offset());
    let remaining = remaining.as_ref();
    let mut len = 0usize;
    let mut size = 0usize;
    let mut is_last = true;

    while let Some(&byte) = remaining.get(len) {
        if !is_hex_digit(byte) {
            break;
        }

        let digit = hex_digit_value(byte).ok_or_else(|| ErrMode::from_input(&*input))?;

        size = size
            .checked_mul(16)
            .and_then(|size| size.checked_add(digit))
            .ok_or_else(|| ErrMode::from_input(&*input))?;
        is_last &= byte == b'0';
        len += 1;
    }

    if len == 0 {
        return Err(ErrMode::from_input(&*input));
    }

    let _ = input.next_slice(len);

    Ok((size, is_last))
}

fn parse_crlf<I>(input: &mut I) -> ModalResult<()>
where
    I: Stream<Token = u8> + StreamIsPartial + Compare<u8>,
{
    b'\r'.parse_next(input)?;
    b'\n'.parse_next(input)?;
    Ok(())
}

/// Returns `true` if the given byte is valid in `HEXDIG`.
///
/// See: [RFC 5234 Appendix B.1](https://datatracker.ietf.org/doc/html/rfc5234#appendix-B.1)
fn is_hex_digit(byte: u8) -> bool {
    byte.is_ascii_hexdigit()
}

/// Returns `true` if the given byte is valid in `tchar`.
///
/// See: [RFC 9110 §5.6.2](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.2)
fn is_tchar(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'a'..=b'z'
    )
}

/// Returns `true` if the given byte is valid in `qdtext`.
///
/// See: [RFC 9110 §5.6.4](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4)
fn is_qdtext_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b' ' | 0x21 | 0x23..=0x5B | 0x5D..=0x7E | 0x80..=0xFF)
}

/// Returns `true` if the given byte is valid in `quoted-pair`.
///
/// See: [RFC 9110 §5.6.4](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.4)
fn is_quoted_pair_byte(byte: u8) -> bool {
    matches!(byte, b'\t' | b' ' | 0x21..=0x7E | 0x80..=0xFF)
}

/// Returns `true` if the given byte is optional whitespace.
///
/// See: [RFC 9110 §5.6.3](https://datatracker.ietf.org/doc/html/rfc9110#section-5.6.3)
fn is_ows_byte(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

fn hex_digit_value(byte: u8) -> Option<usize> {
    Some(match byte {
        b'0'..=b'9' => (byte - b'0') as usize,
        b'a'..=b'f' => (byte - b'a' + 10) as usize,
        b'A'..=b'F' => (byte - b'A' + 10) as usize,
        _ => return None,
    })
}

fn has_chunk_ext_prefix(bytes: &[u8]) -> bool {
    matches!(bytes.iter().position(|&byte| !is_ows_byte(byte)), Some(index) if bytes[index] == b';')
}

#[cfg(test)]
mod tests {
    use winnow::{BStr, prelude::*};

    use super::*;

    macro_rules! assert_ok_peek {
        ($parser:expr, $input:expr, $remaining:expr, $output:expr $(,)?) => {
            assert_eq!(
                $parser.parse_peek(BStr::new($input)),
                Ok((BStr::new($remaining), $output)),
            );
        };
    }

    #[test]
    fn parses_chunk_size_fn() {
        assert_ok_peek!(parse_chunk_size, b"4\r\n", b"\r\n", 4usize);
        assert_ok_peek!(parse_chunk_size, b"000a;foo", b";foo", 10usize);

        assert!(parse_chunk_size.parse(&b"g"[..]).is_err());
    }

    #[test]
    fn parses_chunk_data_fn() {
        assert_ok_peek!(parse_chunk_data(4), b"Wiki\r\n", b"\r\n", ());
        assert_ok_peek!(parse_chunk_data(10), b"0123456789rest", b"rest", ());

        assert!(parse_chunk_data(0).parse(&b""[..]).is_err());
        assert!(parse_chunk_data(4).parse(&b"Wik"[..]).is_err());
    }

    #[test]
    fn parses_chunk_header_fn() {
        assert_ok_peek!(parse_chunk_header, b"4\r\nWiki", b"Wiki", 4usize);
        assert_ok_peek!(
            parse_chunk_header,
            b"000a;foo=bar\r\nbody",
            b"body",
            10usize
        );
        assert_ok_peek!(
            parse_chunk_header,
            b"0;sig=ok\r\ntrailers",
            b"trailers",
            0usize,
        );
    }

    #[test]
    fn parses_chunk_fn() {
        assert_ok_peek!(parse_chunk, b"4\r\nWiki\r\nrest", b"rest", 4usize);
        assert_ok_peek!(parse_chunk, b"a\r\n0123456789\r\nx", b"x", 10usize);

        assert!(parse_chunk.parse(&b"0\r\n"[..]).is_err());
        assert!(parse_chunk.parse(&b"4\r\nWik"[..]).is_err());
    }

    #[test]
    fn parses_last_chunk_fn() {
        assert_ok_peek!(parse_last_chunk, b"0\r\ntrailers", b"trailers", ());
        assert_ok_peek!(parse_last_chunk, b"000;sig=ok\r\ntrailers", b"trailers", ());

        assert!(parse_last_chunk.parse(&b"1\r\n"[..]).is_err());
    }

    #[test]
    fn parses_chunk_extension_fns() {
        assert_ok_peek!(parse_chunk_ext, b"", b"", ());
        assert_ok_peek!(parse_chunk_ext, b";foo=bar; baz = \"qux\"", b"", ());
        assert_ok_peek!(parse_chunk_ext_param, b";foo=bar rest", b" rest", ());
        assert_ok_peek!(parse_chunk_ext_val, b"bar;", b";", ());
        assert_ok_peek!(parse_chunk_ext_val, b"\"qux\";", b";", ());

        assert!(parse_chunk_ext_param.parse(&b"foo=bar"[..]).is_err());
    }

    #[test]
    fn parses_token_and_quoted_forms() {
        assert_ok_peek!(parse_token, b"chunk-signature;", b";", ());
        assert_ok_peek!(parse_quoted_string, b"\"qux\";", b";", ());
        assert_ok_peek!(parse_quoted_pair, b"\\\"rest", b"rest", ());
        assert_ok_peek!(parse_bws, b" \t;foo", b";foo", ());

        assert!(parse_token.parse(&b"bad token"[..]).is_err());
        assert!(parse_quoted_string.parse(&b"\"unterminated"[..]).is_err());
        assert!(parse_quoted_pair.parse(&b"\\"[..]).is_err());
    }
}
