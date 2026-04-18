#![allow(missing_docs)]

use http_request_target::RequestTarget;
use winnow::BStr;

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
        match RequestTarget::try_from_slice($target.as_bytes()) {
            Ok(RequestTarget::Origin(target)) => panic!(
                "unexpectedly parsed {:?} as Origin({:?})",
                BStr::new($target.as_bytes()),
                BStr::new(target.as_bytes()),
            ),
            Ok(RequestTarget::Absolute(target)) => panic!(
                "unexpectedly parsed {:?} as Absolute({:?})",
                BStr::new($target.as_bytes()),
                BStr::new(target.as_bytes()),
            ),
            Ok(RequestTarget::Authority(target)) => panic!(
                "unexpectedly parsed {:?} as Authority({:?})",
                BStr::new($target.as_bytes()),
                BStr::new(target.as_bytes()),
            ),
            Ok(RequestTarget::Asterisk) => panic!(
                "unexpectedly parsed {:?} as Asterisk",
                BStr::new($target.as_bytes()),
            ),
            Err(_) => {}
        };
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
    pass!("git+http://www.example.org/repo");
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
fn origin_components() {
    let target = match RequestTarget::try_from_slice(b"/where?q=now").unwrap() {
        RequestTarget::Origin(target) => target,
        other => panic!("unexpected variant: {other:?}"),
    };

    assert_eq!(target.as_bytes(), b"/where?q=now");
    assert_eq!(target.path_indices(), 0..6);
    assert_eq!(target.path(), b"/where");
    assert_eq!(target.query_indices(), Some(7..12));
    assert_eq!(target.query(), Some(&b"q=now"[..]));
}

#[test]
fn authority_components() {
    let target = match RequestTarget::try_from_slice(b"localhost:3000").unwrap() {
        RequestTarget::Authority(target) => target,
        other => panic!("unexpected variant: {other:?}"),
    };

    assert_eq!(target.as_bytes(), b"localhost:3000");
    assert_eq!(target.host_indices(), 0..9);
    assert_eq!(target.host(), b"localhost");
    assert_eq!(target.port_indices(), 10..14);
    assert_eq!(target.port(), b"3000");
}

#[test]
fn absolute_components() {
    let target =
        match RequestTarget::try_from_slice(b"git+http://user:pass@example.com:1234/repo?q=1")
            .unwrap()
        {
            RequestTarget::Absolute(target) => target,
            other => panic!("unexpected variant: {other:?}"),
        };

    assert_eq!(target.as_bytes(), b"git+http://user:pass@example.com:1234/repo?q=1");
    assert_eq!(target.scheme_indices(), 0..8);
    assert_eq!(target.scheme(), b"git+http");
    assert_eq!(target.authority_indices(), 11..37);
    assert_eq!(target.authority(), b"user:pass@example.com:1234");
    assert_eq!(target.userinfo_indices(), Some(11..20));
    assert_eq!(target.userinfo(), Some(&b"user:pass"[..]));
    assert_eq!(target.host_indices(), 21..32);
    assert_eq!(target.host(), b"example.com");
    assert_eq!(target.port_indices(), Some(33..37));
    assert_eq!(target.port(), Some(&b"1234"[..]));
    assert_eq!(target.path_indices(), 37..42);
    assert_eq!(target.path(), b"/repo");
    assert_eq!(target.query_indices(), Some(43..46));
    assert_eq!(target.query(), Some(&b"q=1"[..]));
}

#[test]
fn fail() {
    fail!("http://");
    fail!("htt:p//host");
    fail!("actix.dev/");
    fail!("actix.dev?key=val");
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
