#[cfg(test)]
mod header {
    use crate::header::names::TEST_HEADERS;
    use crate::{header::HeaderKind, HeaderName, HeaderValue};

    #[test]
    fn parse_standard_headers() {
        for &(std_header, lowercase) in TEST_HEADERS {
            let uppercase = lowercase.to_ascii_uppercase();
            let from_lowercase = HeaderName::try_from(lowercase);
            let from_uppercase = HeaderName::try_from(uppercase.as_slice());
            assert_eq!(Ok(HeaderName::from(std_header)), from_lowercase);
            assert_eq!(Ok(HeaderName::from(std_header)), from_uppercase);
        }
    }

    #[test]
    fn parse_custom_headers() {
        macro_rules! test_custom_headers {
            ( $($name:expr => $val:expr;)+ ) =>  {{
                $(
                    let test_name = HeaderName::try_from($name).unwrap();
                    let exp_kind = HeaderKind::Custom($name.to_owned());
                    let exp_name = HeaderName { inner: exp_kind };
                    assert_eq!(test_name, exp_name);
                )+

                $(
                    let test_val = HeaderValue::from($val);
                    let exp_val = HeaderValue($val.to_owned());
                    assert_eq!(test_val, exp_val);
                )+
            }};
        }

        test_custom_headers! {
            Vec::from("cats").as_slice()  => Vec::from("dogs").as_slice();
            Vec::from("sun").as_slice()   => Vec::from("moon").as_slice();
            Vec::from("black").as_slice() => Vec::from("white").as_slice();
            Vec::from("hot").as_slice()   => Vec::from("cold").as_slice();
            Vec::from("tired").as_slice() => Vec::from("awake").as_slice();
        }
    }
}

#[cfg(test)]
mod http {
    use std::str::FromStr;
    use crate::{Method, Status, Version};

    #[test]
    fn parse_method() {
        let get = "GET".parse::<Method>();
        let head = "HEAD".parse::<Method>();
        let post = "POST".parse::<Method>();
        let put = "PUT".parse::<Method>();
        let patch = "PATCH".parse::<Method>();
        let delete = "DELETE".parse::<Method>();
        let trace = "TRACE".parse::<Method>();
        let options = "OPTIONS".parse::<Method>();
        let connect = "CONNECT".parse::<Method>();
        let bad_get = "get".parse::<Method>();
        let unknown = "FOO".parse::<Method>();

        assert_eq!(get, Ok(Method::Get));
        assert_eq!(head, Ok(Method::Head));
        assert_eq!(post, Ok(Method::Post));
        assert_eq!(put, Ok(Method::Put));
        assert_eq!(patch, Ok(Method::Patch));
        assert_eq!(delete, Ok(Method::Delete));
        assert_eq!(trace, Ok(Method::Trace));
        assert_eq!(options, Ok(Method::Options));
        assert_eq!(connect, Ok(Method::Connect));
        assert!(bad_get.is_err());
        assert!(unknown.is_err());
    }

    #[test]
    fn parse_status() {
        let s100 = "100";
        let s201 = "201";
        let s301 = "301";
        let s403 = "403";
        let s500 = "500";
        let bad = "abc";

        assert_eq!(s100.parse::<Status>(), Ok(Status(100)));
        assert_eq!(s201.parse::<Status>(), Ok(Status(201)));
        assert_eq!(s301.parse::<Status>(), Ok(Status(301)));
        assert_eq!(s403.parse::<Status>(), Ok(Status(403)));
        assert_eq!(s500.parse::<Status>(), Ok(Status(500)));
        assert!(bad.parse::<Status>().is_err());
    }

    #[test]
    fn parse_version() {
        let v0_9 = Version::from_str("HTTP/0.9");
        let v1_0 = Version::from_str("HTTP/1.0");
        let v1_1 = Version::from_str("HTTP/1.1");
        let v2_0 = Version::from_str("HTTP/2.0");
        let v3_0 = Version::from_str("HTTP/3.0");
        let bad = Version::from_str("HTTP/1.2");

        assert_eq!(v0_9, Ok(Version::ZeroDotNine));
        assert_eq!(v1_0, Ok(Version::OneDotZero));
        assert_eq!(v1_1, Ok(Version::OneDotOne));
        assert_eq!(v2_0, Ok(Version::TwoDotZero));
        assert_eq!(v3_0, Ok(Version::ThreeDotZero));
        assert!(bad.is_err());
    }
}

#[cfg(test)]
mod client {
    use crate::{
        Client, Method, HeaderName, Header, Headers, HeaderValue,
        RequestLine, Version,
    };
    use crate::consts::{ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, USER_AGENT};
    use crate::header::names::HeaderKind;
    use std::collections::BTreeMap;

    #[test]
    fn request_line() {
        let uri = "/test";
        let test1 = RequestLine::parse("GET /test HTTP/1.1\r\n").unwrap();
        let test2 = RequestLine::parse("HEAD /test HTTP/1.1\r\n").unwrap();
        let test3 = RequestLine::parse("POST /test HTTP/1.1\r\n").unwrap();
        let test4 = RequestLine::parse("PUT /test HTTP/1.1\r\n").unwrap();
        let test5 = RequestLine::parse("PATCH /test HTTP/1.1\r\n").unwrap();
        let test6 = RequestLine::parse("DELETE /test HTTP/1.1\r\n").unwrap();
        let test7 = RequestLine::parse("TRACE /test HTTP/1.1\r\n").unwrap();
        let test8 = RequestLine::parse("OPTIONS /test HTTP/1.1\r\n").unwrap();
        let test9 = RequestLine::parse("CONNECT /test HTTP/1.1\r\n").unwrap();
        let bad1 = RequestLine::parse("GET");
        let bad2 = RequestLine::parse("GET /test");
        let bad3 = RequestLine::parse("FOO bar baz");

        let expected1 = RequestLine::new(Method::Get, uri.to_string(), Version::OneDotOne);
        let expected2 = RequestLine::new(Method::Head, uri.to_string(), Version::OneDotOne);
        let expected3 = RequestLine::new(Method::Post, uri.to_string(), Version::OneDotOne);
        let expected4 = RequestLine::new(Method::Put, uri.to_string(), Version::OneDotOne);
        let expected5 = RequestLine::new(Method::Patch, uri.to_string(), Version::OneDotOne);
        let expected6 = RequestLine::new(Method::Delete, uri.to_string(), Version::OneDotOne);
        let expected7 = RequestLine::new(Method::Trace, uri.to_string(), Version::OneDotOne);
        let expected8 = RequestLine::new(Method::Options, uri.to_string(), Version::OneDotOne);
        let expected9 = RequestLine::new(Method::Connect, uri.to_string(), Version::OneDotOne);

        assert_eq!(test1, expected1);
        assert_eq!(test2, expected2);
        assert_eq!(test3, expected3);
        assert_eq!(test4, expected4);
        assert_eq!(test5, expected5);
        assert_eq!(test6, expected6);
        assert_eq!(test7, expected7);
        assert_eq!(test8, expected8);
        assert_eq!(test9, expected9);
        assert!(bad1.is_err());
        assert!(bad2.is_err());
        assert!(bad3.is_err());
    }

    #[test]
    fn parse_headers() {
        let test = "\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n";

        let expected = Headers(BTreeMap::from([
            (ACCEPT, HeaderValue::from("*/*")),
            (ACCEPT_ENCODING, HeaderValue::from("gzip, deflate, br")),
            (CONNECTION, HeaderValue::from("keep-alive")),
            (HOST, HeaderValue::from("example.com")),
            (USER_AGENT, HeaderValue::from("xh/0.19.3")),
            (
                HeaderName {
                    inner: HeaderKind::Custom(Vec::from("Pineapple")),
                },
                HeaderValue::from("pizza"),
            ),
        ]));

        let mut output = Headers::new();

        for line in test.split('\n') {
            let trim = line.trim();
            if trim.is_empty() {
                break;
            }

            let (name, value) = Header::parse(trim).unwrap();
            output.insert(name, value);
        }

        assert_eq!(output.0.len(), expected.0.len());
        assert!(output.0
            .iter()
            .zip(expected.0)
            .all(|((k_out, v_out), (k_exp, v_exp))| {
                *k_out == k_exp && *v_out == v_exp 
            })
        );
    }

    #[test]
    fn parse_uri() {
        macro_rules! test_uri_parser {
            ( $(SHOULD_ERROR: $uri:literal;)+ ) => {{
                $(
                    let parse_result = Client::parse_uri($uri);
                    assert!(parse_result.is_err());
                )+
            }};
            ( $($uri:literal: $addr:literal, $path:literal;)+ ) => {{
                $(
                    let (test_addr, test_path) = Client::parse_uri($uri).unwrap();
                    assert_eq!(test_addr, $addr);
                    assert_eq!(test_path, $path);
                )+
            }};
        }

        test_uri_parser! {
            "http://www.example.com/test.html": "www.example.com:80", "/test.html";
            "http://www.example.com/": "www.example.com:80", "/";
            "http://example.com/": "example.com:80", "/";
            "http://example.com": "example.com:80", "/";
            "www.example.com/test.html": "www.example.com:80", "/test.html";
            "www.example.com/": "www.example.com:80", "/";
            "example.com/test.html": "example.com:80", "/test.html";
            "example.com/": "example.com:80", "/";
            "example.com": "example.com:80", "/";
            "www.example.com:80/test": "www.example.com:80", "/test";
            "127.0.0.1:80/test": "127.0.0.1:80", "/test";
        }

        test_uri_parser! {
            SHOULD_ERROR: "https://www.example.com";
            SHOULD_ERROR: "http://";
        }
    }
}

#[cfg(test)]
mod utils {
    use crate::trim_whitespace_bytes;

    #[test]
    fn trim_whitespace() {
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
}

#[cfg(test)]
mod router {
    use std::collections::BTreeMap;
    use crate::{
        Method::*, Request, RequestLine, Router, Route, Resolved, Status,
        Target::*, Version, Headers,
    };

    macro_rules! test_routes {
        ($($method:ident $path:literal => $target:expr, $status:expr;)+) => {
            #[test]
            fn resolve_requests() {
                let routes = BTreeMap::from([
                    $( (Route::new($method, $path), $target) ),+
                ]);

                let router = Router(routes);

                $(
                    let req = Request {
                        request_line: RequestLine {
                            method: $method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: None
                    };

                    let expected = Resolved {
                        status: $status,
                        method: $method,
                        target: $target
                    };

                    assert_eq!(router.resolve(&req, false), expected);
                )+
            }
        };
    }

    test_routes! {
        Get "/test1" => File("test1.html".into()), Status(200);
        Head "/test2" => File("test2.html".into()), Status(200);
        Post "/test3" => File("test3.html".into()), Status(200);
        Put "/test4" => File("test4.html".into()), Status(200);
        Patch "/test5" => File("test5.html".into()), Status(200);
        Delete "/test6" => File("test6.html".into()), Status(200);
        Trace "/test7" => File("test7.html".into()), Status(200);
        Options "/test8" => File("test8.html".into()), Status(200);
        Connect "127.0.0.1:1234" => Text("connected".into()), Status(200);
    }
}
