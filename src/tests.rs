#[test]
fn test_request_headers_search() {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use crate::{Header, HeaderName, Request, RequestLine};

    let cache = Header {
        name: HeaderName::CacheControl,
        value: "no-cache".to_string()
    };
    let c_type = Header {
        name: HeaderName::ContentType,
        value: "text/plain; charset=UTF-8".to_string()
    };
    let c_len = Header {
        name: HeaderName::ContentLength,
        value: "0".to_string()
    };
    let headers = vec![cache.clone(), c_len.clone(), c_type.clone()];

    let remote_addr = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        8080
    );

    let req = Request {
        remote_addr,
        request_line: RequestLine::default(),
        headers,
        body: Vec::new()
    };

    assert_eq!(req.get_header(&HeaderName::CacheControl).unwrap(), &cache);
    assert_eq!(req.get_header(&HeaderName::ContentLength).unwrap(), &c_len);
    assert_eq!(req.get_header(&HeaderName::ContentType).unwrap(), &c_type);
    assert_ne!(req.get_header(&HeaderName::ContentType).unwrap(), &c_len);
    assert!(!req.has_header(&HeaderName::Host));
}

#[test]
fn test_parse_request_line() {
    use std::str::FromStr;
    use crate::{RequestLine, Method::*, Version::*};

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
    use std::str::FromStr;
    use crate::{Header, HeaderName};

    let test_headers = "\
        Accept: */*\r\n\
        Accept-Encoding: gzip, deflate, br\r\n\
        Connection: keep-alive\r\n\
        Host: example.com\r\n\
        User-Agent: xh/0.19.3\r\n\
        Pineapple: pizza\r\n\r\n";

    let expected = vec![
        Header { name: HeaderName::Accept, value: "*/*".to_string() },
        Header { name: HeaderName::AcceptEncoding, value: "gzip, deflate, br".to_string() },
        Header { name: HeaderName::Connection, value: "keep-alive".to_string() },
        Header { name: HeaderName::Host, value: "example.com".to_string() },
        Header { name: HeaderName::UserAgent, value: "xh/0.19.3".to_string() },
        Header { name: HeaderName::Unknown("pineapple".to_string()), value: "pizza".to_string() }
    ];

    let mut output = vec![];

    for line in test_headers.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }

        output.push(Header::from_str(trimmed).unwrap());
    }

    assert_eq!(&output[..], &expected[..]);
}

#[test]
fn test_response_headers_search() {
    use crate::{Header, HeaderName, Response};

    let res = Response::default();

    let cache_con = Header {
        name: HeaderName::CacheControl,
        value: "no-cache".to_string()
    };
    let cont_len = Header {
        name: HeaderName::ContentLength,
        value: "0".to_string()
    };
    let cont_type = Header {
        name: HeaderName::ContentType,
        value: "text/plain; charset=UTF-8".to_string()
    };

    assert_eq!(res.get_header(&HeaderName::CacheControl).unwrap(), &cache_con);
    assert_eq!(res.get_header(&HeaderName::ContentLength).unwrap(), &cont_len);
    assert_eq!(res.get_header(&HeaderName::ContentType).unwrap(), &cont_type);
    assert_ne!(res.get_header(&HeaderName::ContentType).unwrap(), &cont_len);
    assert!(!res.has_header(&HeaderName::Host));
}

#[test]
fn test_trim_whitespace_bytes() {
    use crate::util::trim_whitespace;

    assert_eq!(trim_whitespace(b"  test"), b"test");
    assert_eq!(trim_whitespace(b"test    "), b"test");
    assert_eq!(trim_whitespace(b"         test       "), b"test");
    assert_eq!(trim_whitespace(b"  Hello \nworld       "), b"Hello \nworld");
    assert_eq!(trim_whitespace(b"\t  \nx\t  x\r\x0c"), b"x\t  x");
    assert_eq!(trim_whitespace(b"                   "), b"");
    assert_eq!(trim_whitespace(b" "), b"");
    assert_eq!(trim_whitespace(b"x"), b"x");
    assert_eq!(trim_whitespace(b""), b"");
}
