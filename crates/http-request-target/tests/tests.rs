#![allow(missing_docs)]

use http_request_target::RequestTarget;

macro_rules! pass {
    ($target:literal) => {
        RequestTarget::try_from_slice($target.as_bytes()).unwrap();
    };
}

#[test]
fn pass() {
    // origin form
    pass!("/");
    pass!("/just/path");
    pass!("/path?with=query");
    pass!("/some/path/here?and=then&hello#and-bye");
    pass!("/echo/abcdefgh_i-j%20/abcdefg_i-j%20478");
    pass!("/foo=bar|baz\\^~%");
    pass!("/?foo={bar|baz}\\^`");

    // absolute form

    // authority-form

    // asterisk form
    pass!("*");

    // pass!("http://127.0.0.1:61761/chunks");
    // pass!("https://127.0.0.1:61761");
    // pass!("localhost");
    // pass!("S");
    // pass!("localhost:3000");
    // pass!("http://127.0.0.1:80");
    // pass!("https://127.0.0.1:443");
    // pass!("http://127.0.0.1/#?");
    // pass!("http://127.0.0.1/path?");
    // pass!("http://127.0.0.1?foo=bar");
    // pass!("http://127.0.0.1#foo/bar");
    // pass!("http://127.0.0.1#foo?bar");
    // pass!("thequickbrownfoxjumpedoverthelazydogtofindthelargedangerousdragon.localhost");
    // pass!("thequickbrownfoxjumpedoverthelazydogtofindthelargedangerousdragon.localhost:1234");
    // pass!("http://a:b@127.0.0.1:1234/");
    // pass!("http://a:b@127.0.0.1/");
    // pass!("http://a@127.0.0.1/");
    // pass!("user@localhost:3000");
    // pass!("user:pass@localhost:3000");
    // pass!("http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]/");
    // pass!("http://[::1]/");
    // pass!("http://[::]/");
    // pass!("http://[2001:db8::2:1]/");
    // pass!("http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8008/");
}

macro_rules! fail {
    ($target:literal) => {
        RequestTarget::try_from_slice($target.as_bytes()).unwrap_err();
    };
}

#[test]
#[ignore]
fn fail() {
    fail!("http://");
    fail!("htt:p//host");
    fail!("hyper.rs/");
    fail!("hyper.rs?key=val");
    fail!("?key=val");
    fail!("localhost/");
    fail!("localhost?key=val");
    fail!("\0");
    fail!("http://[::1");
    fail!("http://::1]");
    fail!("localhost:8080:3030");
    fail!("@");
    fail!("http://username:password@/wut");
    fail!("/?foo\rbar");
    fail!("/?foo\nbar");
    fail!("/?<");
    fail!("/?>");
}
