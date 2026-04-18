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
fn absolute_form() {}

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
