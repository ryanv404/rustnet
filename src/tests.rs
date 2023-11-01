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

    use crate::consts::{ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, USER_AGENT};
    use crate::header::names::HdrRepr;
    use crate::{HeaderName, Request};

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
            HeaderName {
                inner: HdrRepr::Custom(Vec::from("pineapple")),
            },
            "pizza".into(),
        ),
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
    assert_eq!(
        trim_whitespace_bytes(b"  Hello \nworld       "),
        b"Hello \nworld"
    );
    assert_eq!(trim_whitespace_bytes(b"\t  \nx\t  x\r\x0c"), b"x\t  x");
    assert_eq!(trim_whitespace_bytes(b"                   "), b"");
    assert_eq!(trim_whitespace_bytes(b" "), b"");
    assert_eq!(trim_whitespace_bytes(b"x"), b"x");
    assert_eq!(trim_whitespace_bytes(b""), b"");
}

#[test]
fn test_client_response_parsing() {
    use crate::{Client, HeaderValue as HdrVal, Status, Version};
    use crate::consts::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_ORIGIN, CONNECTION,
        CONTENT_LENGTH, CONTENT_TYPE, DATE, LOCATION, SERVER
    };

    let addr = "httpbin.org:80";
    let test_codes: [u16; 5] = [101, 202, 303, 404, 505];

    for code in test_codes.into_iter() {
        // Responds with the status code corresponding to `code`.
        let uri = format!("/status/{code}");
        let mut client = Client::http().addr(&addr).uri(&uri).build().unwrap();

        println!("{}", &client);
        client.send().unwrap();
        let res = client.recv().unwrap();
        println!("{}", &res);

        assert_eq!(res.version, Version::OneDotOne);
        assert_eq!(res.status, Status(code));
        assert_eq!(
            res.headers.get(&ACCESS_CONTROL_ALLOW_CREDENTIALS),
            Some(&HdrVal::new(b"true"))
        );
        assert_eq!(
            res.headers.get(&ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HdrVal::new(b"*"))
        );

        if code == 101 {
            assert_eq!(
                res.headers.get(&CONNECTION),
                Some(&HdrVal::new(b"upgrade"))
            );
        } else  {
            assert_eq!(
                res.headers.get(&CONNECTION),
                Some(&HdrVal::new(b"keep-alive"))
            );
            assert_eq!(
                res.headers.get(&CONTENT_LENGTH),
                Some(&HdrVal::new(b"0"))
            );
        }

        if code == 303 {
            assert_eq!(
                res.headers.get(&LOCATION),
                Some(&HdrVal::new(b"/redirect/1"))
            );
        } else {
            assert_eq!(
                res.headers.get(&CONTENT_TYPE),
                Some(&HdrVal::new(b"text/html; charset=utf-8"))
            );
        }

        assert_eq!(
            res.headers.get(&SERVER),
            Some(&HdrVal::new(b"gunicorn/19.9.0"))
        );
        assert!(res.headers.contains_key(&DATE));
        //assert_eq!(res.body, Vec::new());
    }
}
