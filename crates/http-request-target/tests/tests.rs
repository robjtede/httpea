#![allow(missing_docs)]

use http_request_target::RequestTarget;

macro_rules! pass {
    ($target:literal) => {
        match RequestTarget::try_from_slice($target.as_bytes()) {
            Ok(_) => {}
            Err(err) => panic!("{:?} {:?}", err.input(), err),
        };
    };
}

macro_rules! fail {
    ($target:literal) => {
        RequestTarget::try_from_slice($target.as_bytes()).unwrap_err();
    };
}

#[test]
fn origin_form() {
    pass!("/");
    pass!("/just/path");
    pass!("/path?with=query");
    // pass!("/some/path/here?and=then&hello#and-bye");
    pass!("/echo/abcdefgh_i-j%20/abcdefg_i-j%20478");
    // pass!("/foo=bar|baz\\^~%");
    // pass!("/?foo={bar|baz}\\^`");
}

#[test]
fn absolute_form() {
    pass!("http://www.example.org:61761/chunks");
    pass!("https://www.example.org:61761");
    pass!("http://127.0.0.1:61761/chunks");
    pass!("http://www.example.org:80");
    pass!("https://www.example.org:443");
    pass!("http://127.0.0.1:80");
    pass!("http://www.example.org?foo=bar");
    pass!("http://www.example.org/path?");
    pass!("http://a:b@www.example.org:1234/");
    pass!("http://a:b@www.example.org/");
    pass!("http://a@www.example.org/");
    pass!("http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]/");
    pass!("http://[::1]/");
    pass!("http://[::]/");
    pass!("http://[2001:db8::2:1]/");
    pass!("http://[2001:0db8:85a3:0000:0000:8a2e:0370:7334]:8008/");
}

#[test]
fn asterisk_form() {
    pass!("*");
}

#[test]
fn authority_form() {
    pass!("localhost:3000");
    pass!("127.0.0.1:80");
    pass!("[::1]:443");
    pass!("thequickbrownfoxjumpedoverthelazydogtofindthelargedangerousdragon.localhost:1234");
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
    fail!("localhost");
    fail!("[::1]");
    fail!("user@localhost:3000");
    fail!("/?foo\rbar");
    fail!("/?foo\nbar");
    fail!("/?<");
    fail!("/?>");
}
