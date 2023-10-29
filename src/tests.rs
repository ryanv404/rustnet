#[test]
fn test_parse_request_line() {
    use crate::{Method::*, Request, Version::*};

    let test1 = "GET /test HTTP/1.1";
    let test2 = "POST /test HTTP/2.0";
    let test3 = "   GET /test HTTP/1.1    ";
    let test4 = "foo bar baz";
    let test5 = "GET /test";
    let test6 = "GET";

	let expected1 = (Get, "/test".to_owned(), OneDotOne);
    let expected2 = (Post, "/test".to_owned(), TwoDotZero);

    assert_eq!(Request::parse_request_line(test1).unwrap(), expected1);
    assert_eq!(Request::parse_request_line(test2).unwrap(), expected2);
    assert_eq!(Request::parse_request_line(test3).unwrap(), expected1);
    assert!(Request::parse_request_line(test4).is_err());
    assert!(Request::parse_request_line(test5).is_err());
    assert!(Request::parse_request_line(test6).is_err());
}

#[test]
fn test_parse_request_headers() {
	use std::collections::BTreeMap;

	use crate::{HeaderName, Request};
	use crate::header::names::HdrRepr;
    use crate::consts::{ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, USER_AGENT};

    let test = "\
        Accept: */*\r\n\
        Accept-Encoding: gzip, deflate, br\r\n\
        Connection: keep-alive\r\n\
        Host: example.com\r\n\
        User-Agent: xh/0.19.3\r\n\
        Pineapple: pizza\r\n\r\n";

    let expected = BTreeMap::from([
        (ACCEPT, "*/*".into()),
        (ACCEPT_ENCODING, "gzip, deflate, br".into()),
        (CONNECTION, "keep-alive".into()),
        (HOST, "example.com".into()),
        (USER_AGENT, "xh/0.19.3".into()),
        (
			HeaderName{ inner: HdrRepr::Custom(Vec::from("pineapple")) },
			"pizza".into()
        )
    ]);

    let mut output = BTreeMap::new();

    for line in test.split('\n') {
        let trim = line.trim();

		if trim.is_empty() {
            break;
        }

        let (name, value) = Request::parse_header(trim).unwrap();
        output.insert(name, value);
    }

    assert_eq!(output, expected);
}

#[test]
fn test_trim_whitespace_bytes() {
    use crate::trim_whitespace_bytes;

    assert_eq!(trim_whitespace_bytes(b"  test"), b"test");
    assert_eq!(trim_whitespace_bytes(b"test    "), b"test");
    assert_eq!(trim_whitespace_bytes(b"         test       "), b"test");
    assert_eq!(trim_whitespace_bytes(b"  Hello \nworld       "), b"Hello \nworld");
    assert_eq!(trim_whitespace_bytes(b"\t  \nx\t  x\r\x0c"), b"x\t  x");
    assert_eq!(trim_whitespace_bytes(b"                   "), b"");
    assert_eq!(trim_whitespace_bytes(b" "), b"");
    assert_eq!(trim_whitespace_bytes(b"x"), b"x");
    assert_eq!(trim_whitespace_bytes(b""), b"");
}
