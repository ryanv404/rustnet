#[test]
fn test_request_headers_search() {
    use crate::{
        Header, Request, RequestLine,
        consts::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, HOST},
    };
    use std::{collections::HashMap, net::{IpAddr, Ipv4Addr, SocketAddr}};

    let cache = Header {
        name: CACHE_CONTROL,
        value: "no-cache".to_string(),
    };
    let c_len = Header {
        name: CONTENT_LENGTH,
        value: "0".to_string(),
    };
    let c_type = Header {
        name: CONTENT_TYPE,
        value: "text/plain; charset=UTF-8".to_string(),
    };

    let headers = HashMap::from([
        (cache.name.clone(), cache.clone()),
        (c_len.name.clone(), c_len.clone()),
        (c_type.name.clone(), c_type.clone())
    ]);

    let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);

    let req = Request {
        remote_addr,
        request_line: RequestLine::default(),
        headers,
        body: Vec::new(),
    };

    assert_eq!(req.get_header(&CACHE_CONTROL).unwrap(), &cache);
    assert_eq!(req.get_header(&CONTENT_LENGTH).unwrap(), &c_len);
    assert_eq!(req.get_header(&CONTENT_TYPE).unwrap(), &c_type);
    assert_ne!(req.get_header(&CONTENT_TYPE).unwrap(), &c_len);
    assert!(!req.has_header(&HOST));
}

#[test]
fn test_parse_request_line() {
    use crate::{Method::*, RequestLine, Version::*};
    use std::str::FromStr;

    let expected1 = RequestLine::new(Get, "/test", OneDotOne);
    let expected2 = RequestLine::new(Post, "/test", TwoDotZero);
    let test1 = "GET /test HTTP/1.1";
    let test2 = "POST /test HTTP/2.0";
    let test3 = "   GET /test HTTP/1.1    ";
    let test4 = "foo bar baz";
    let test5 = "GET /test";
    let test6 = "GET";

    assert_eq!(RequestLine::from_str(test1).unwrap(), expected1);
    assert_eq!(RequestLine::from_str(test2).unwrap(), expected2);
    assert_eq!(RequestLine::from_str(test3).unwrap(), expected1);
    assert!(RequestLine::from_str(test4).is_err());
    assert!(RequestLine::from_str(test5).is_err());
    assert!(RequestLine::from_str(test6).is_err());
}

#[test]
fn test_parse_request_headers() {
    use crate::{Header, HeaderName,
        consts::{ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, USER_AGENT},
    };
    use std::{collections::HashMap, str::FromStr};

    let test_headers = "\
        Accept: */*\r\n\
        Accept-Encoding: gzip, deflate, br\r\n\
        Connection: keep-alive\r\n\
        Host: example.com\r\n\
        User-Agent: xh/0.19.3\r\n\
        Pineapple: pizza\r\n\r\n";

    let pineapple = HeaderName::from_str("Pineapple").unwrap();

    let expected = HashMap::from([
        (ACCEPT, Header {
            name: ACCEPT,
            value: "*/*".to_string(),
        }),
        (ACCEPT_ENCODING, Header {
            name: ACCEPT_ENCODING,
            value: "gzip, deflate, br".to_string(),
        }),
        (CONNECTION, Header {
            name: CONNECTION,
            value: "keep-alive".to_string(),
        }),
        (HOST, Header {
            name: HOST,
            value: "example.com".to_string(),
        }),
        (USER_AGENT, Header {
            name: USER_AGENT,
            value: "xh/0.19.3".to_string(),
        }),
        (pineapple.clone(), Header {
            name: pineapple,
            value: "pizza".to_string(),
        })
    ]);

    let mut output = HashMap::new();

    for line in test_headers.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }

        let header = Header::from_str(trimmed).unwrap();
        output.insert(header.name.clone(), header);
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
