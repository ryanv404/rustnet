use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::{
    Body, Client, Connection, Header, HeaderKind, HeaderName, HeaderValue,
    Headers, Method, NetError, NetReader, NetResult, NetWriter, NetParseError,
    Request, RequestLine, Response, Route, RouteBuilder, Router, Server,
    ServerBuilder, ServerConfig, ServerHandle, Status, StatusLine, Target,
    ThreadPool, Version, Worker,
};
use crate::header::{
    ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, USER_AGENT,
    names::STANDARD_HEADERS,
};
use crate::util::trim_whitespace_bytes;

#[cfg(test)]
mod http_method {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!("GET".parse::<Method>(), Ok(Method::Get));
        assert_eq!("HEAD".parse::<Method>(), Ok(Method::Head));
        assert_eq!("POST".parse::<Method>(), Ok(Method::Post));
        assert_eq!("PUT".parse::<Method>(), Ok(Method::Put));
        assert_eq!("PATCH".parse::<Method>(), Ok(Method::Patch));
        assert_eq!("DELETE".parse::<Method>(), Ok(Method::Delete));
        assert_eq!("TRACE".parse::<Method>(), Ok(Method::Trace));
        assert_eq!("OPTIONS".parse::<Method>(), Ok(Method::Options));
        assert_eq!("CONNECT".parse::<Method>(), Ok(Method::Connect));
        assert!("FOO".parse::<Method>().is_err());
        // HTTP methods are case-sensitive.
        assert!("get".parse::<Method>().is_err());
    }
}

#[cfg(test)]
mod http_status {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!("101 Switching Protocols".parse::<Status>(), Ok(Status(101)));
        assert_eq!("201 Created".parse::<Status>(), Ok(Status(201)));
        assert_eq!("300 Multiple Choices".parse::<Status>(), Ok(Status(300)));
        assert_eq!("400 Bad Request".parse::<Status>(), Ok(Status(400)));
        assert_eq!("501 Not Implemented".parse::<Status>(), Ok(Status(501)));
        assert!("abc".parse::<Status>().is_err());
    }

    #[test]
    fn from_int() {
        assert_eq!(Status::try_from(101_u16), Ok(Status(101)));
        assert_eq!(Status::try_from(101_u32), Ok(Status(101)));
        assert_eq!(Status::try_from(101_i32), Ok(Status(101)));
    }
}

#[cfg(test)]
mod http_version {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!("HTTP/0.9".parse::<Version>(), Ok(Version::ZeroDotNine));
        assert_eq!("HTTP/1.0".parse::<Version>(), Ok(Version::OneDotZero));
        assert_eq!("HTTP/1.1".parse::<Version>(), Ok(Version::OneDotOne));
        assert_eq!("HTTP/2.0".parse::<Version>(), Ok(Version::TwoDotZero));
        assert_eq!("HTTP/3.0".parse::<Version>(), Ok(Version::ThreeDotZero));
        assert!("HTTP/1.2".parse::<Version>().is_err());
    }
}

#[cfg(test)]
mod request_line {
    use super::*;

    macro_rules! parse_request_line {
        (SHOULD_ERR: $line:literal) => {
            let should_err = $line.parse::<RequestLine>();
            assert!(should_err.is_err());
        };
        ($method:ident: $line:literal) => {
            let req_line = $line.parse::<RequestLine>();
            let expected = RequestLine {
                method: Method::$method,
                path: "/test".to_string(),
                version: Version::OneDotOne
            };
            assert_eq!(req_line, Ok(expected));
        };
    }

    #[test]
    fn from_str() {
        parse_request_line!(Get: "GET /test HTTP/1.1\r\n");
        parse_request_line!(Head: "HEAD /test HTTP/1.1\r\n");
        parse_request_line!(Post: "POST /test HTTP/1.1\r\n");
        parse_request_line!(Put: "PUT /test HTTP/1.1\r\n");
        parse_request_line!(Patch: "PATCH /test HTTP/1.1\r\n");
        parse_request_line!(Delete: "DELETE /test HTTP/1.1\r\n");
        parse_request_line!(Trace: "TRACE /test HTTP/1.1\r\n");
        parse_request_line!(Options: "OPTIONS /test HTTP/1.1\r\n");
        parse_request_line!(Connect: "CONNECT /test HTTP/1.1\r\n");
        parse_request_line!(SHOULD_ERR: "GET");
        parse_request_line!(SHOULD_ERR: "GET /test");
        parse_request_line!(SHOULD_ERR: "FOO bar baz");
    }
}

#[cfg(test)]
mod status_line {
    use super::*;

    #[test]
    fn from_str() {
        macro_rules! parse_status_line {
            (SHOULD_ERR: $line:literal) => {
                let should_err = $line.parse::<StatusLine>();
                assert!(should_err.is_err());
            };
            ($code:literal: $line:literal) => {
                let status_line = $line.parse::<StatusLine>().unwrap();
                assert_eq!(status_line.version, Version::OneDotOne);
                assert_eq!(status_line.status, Status($code));
            };
        }

        parse_status_line!(100: "HTTP/1.1 100 Continue\r\n");
        parse_status_line!(200: "HTTP/1.1 200 OK\r\n");
        parse_status_line!(301: "HTTP/1.1 301 Moved Permanently\r\n");
        parse_status_line!(403: "HTTP/1.1 403 Forbidden\r\n");
        parse_status_line!(505: "HTTP/1.1 505 HTTP Version Not Supported\r\n");
        parse_status_line!(SHOULD_ERR: "HTTP/1.1");
        parse_status_line!(SHOULD_ERR: "200 OK");
        parse_status_line!(SHOULD_ERR: "FOO bar baz");
    }
}

#[cfg(test)]
mod standard_header {
    use super::*;

    #[test]
    fn from_str() {
        for &(std_header, lowercase) in STANDARD_HEADERS {
            let lower = String::from_utf8(lowercase.to_vec()).unwrap();
            let upper = lower.to_ascii_uppercase();
            let expected = HeaderName {
                inner: HeaderKind::Standard(std_header),
            };
            assert_eq!(lower.parse::<HeaderName>(), Ok(expected.clone()));
            assert_eq!(upper.parse::<HeaderName>(), Ok(expected));
        }
    }
}

#[cfg(test)]
mod custom_header {
    use super::*;

    #[test]
    fn from_str() {
        macro_rules! test_custom_headers {
            ($name:literal, $value:literal) => {{
                let test_name = $name.parse::<HeaderName>().unwrap();
                let expected_name = HeaderName {
                    inner: HeaderKind::Custom(Vec::from($name)),
                };

                let test_value = $value.parse::<HeaderValue>().unwrap();
                let expected_value = HeaderValue(Vec::from($value));

                assert_eq!(test_name, expected_name);
                assert_eq!(test_value, expected_value);
            }};
        }

        test_custom_headers!("Cat", "dog");
        test_custom_headers!("Sun", "moon");
        test_custom_headers!("Black", "white");
        test_custom_headers!("Hot", "cold");
        test_custom_headers!("Tired", "awake");
    }
}

#[cfg(test)]
mod many_headers {
    use super::*;

    #[test]
    fn from_str() {
        let headers_section = "\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n";

        let expected_hdrs = Headers(BTreeMap::from([
            (ACCEPT, b"*/*"[..].into()),
            (ACCEPT_ENCODING, b"gzip, deflate, br"[..].into()),
            (CONNECTION, b"keep-alive"[..].into()),
            (HOST, b"example.com"[..].into()),
            (USER_AGENT, b"xh/0.19.3"[..].into()),
            (
                HeaderName {
                    inner: HeaderKind::Custom(Vec::from("Pineapple")),
                },
                b"pizza"[..].into(),
            ),
        ]));

        let mut test_hdrs = Headers::new();

        for line in headers_section.split('\n') {
            let trim = line.trim();

            if trim.is_empty() {
                break;
            }

            let header = trim.parse::<Header>().unwrap();
            test_hdrs.insert(header.name, header.value);
        }

        assert_eq!(test_hdrs, expected_hdrs);
    }
}

#[cfg(test)]
mod http_uri {
    use super::*;

    #[test]
    fn from_str() {
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
    use super::*;

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
mod resolve_routes {
    use super::*;

    macro_rules! test_route_resolver {
        (empty_targets:
            $(
                $method:ident $path:literal => $code:literal;
            )+
        ) => {
            #[test]
            fn empty_targets() {
                let routes = BTreeMap::from([
                    $( (Route::$method($path.into()), Target::Empty) ),+
                ]);

                let router = Arc::new(Router(routes));

                $(
                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: Body::Empty
                    };

                    let res = Response::for_route(&req.route(), &router);

                    let mut expect = Response {
                        status_line: StatusLine {
                            version: Version::OneDotOne,
                            status: Status($code)
                        },
                        headers: Headers::new(),
                        body: Body::Empty
                    };

                    expect.headers.cache_control("no-cache");
                    assert_eq!(res, Ok(expect));
                )+
            }
        };
        (html_targets:
            $(
                $method:ident $path:literal => $file:literal, $code:literal;
            )+
        ) => {
            #[test]
            fn html_targets() {
                $(
                    let filepath = concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/static/",
                        $file
                    );

                    let route = Route::$method($path.into());
                    let target = Target::File(Path::new(filepath));

                    let mut routes = BTreeMap::new();
                    routes.insert(route, target);
                    let router = Arc::new(Router(routes));

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: Body::Empty
                    };

                    let body = match fs::read(filepath) {
                        Ok(content) => Body::Html(content),
                        Err(e) => panic!(
                            "{e}\nError accessing HTML file at: {}",
                            filepath),
                    };

                    let mut headers = Headers::new();
                    headers.cache_control("no-cache");
                    headers.content_type(
                        "text/html; charset=utf-8"
                    );
                    headers.content_length(body.len());

                    let expect = Response {
                        status_line: StatusLine {
                            version: Version::OneDotOne,
                            status: Status($code)
                        },
                        headers,
                        body: if req.method() == Method::Head {
                            Body::Empty
                        } else {
                            body
                        }
                    };

                    let res = Response::for_route(&req.route(), &router);
                    assert_eq!(res, Ok(expect));
                )+
            }
        };
        (favicon_targets:
            $(
                $method:ident $path:literal => $file:literal, $code:literal;
            )+
        ) => {
            #[test]
            fn favicon_targets() {
                $(
                    let filepath = concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/static/",
                        $file
                    );

                    let route = Route::$method($path.into());
                    let target = Target::Favicon(Path::new(filepath));

                    let mut routes = BTreeMap::new();
                    routes.insert(route, target);
                    let router = Arc::new(Router(routes));

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: Body::Empty
                    };

                    let body = match fs::read(filepath) {
                        Ok(content) => Body::Favicon(content),
                        Err(e) => panic!(
                            "{e}\nError accessing HTML file at: {}",
                            filepath),
                    };

                    let mut headers = Headers::new();
                    headers.cache_control("max-age=604800");
                    headers.content_type("image/x-icon");
                    headers.content_length(body.len());

                    let expect = Response {
                        status_line: StatusLine {
                            version: Version::OneDotOne,
                            status: Status($code)
                        },
                        headers,
                        body: if req.method() == Method::Head {
                            Body::Empty
                        } else {
                            body
                        }
                    };

                    let res = Response::for_route(&req.route(), &router);
                    assert_eq!(res, Ok(expect));
                )+
            }
        };
        (file_targets:
            $(
                $method:ident $path:literal => $file:literal, $code:literal;
            )+
        ) => {
            #[test]
            fn file_targets() {
                $(
                    let filepath = concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/static/",
                        $file
                    );

                    let route = Route::$method($path.into());
                    let target = Target::File(Path::new(filepath));

                    let mut routes = BTreeMap::new();
                    routes.insert(route, target);
                    let router = Arc::new(Router(routes));

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: Body::Empty
                    };

                    let body = match fs::read(filepath) {
                        Ok(content) => Body::Bytes(content),
                        Err(e) => panic!(
                            "{e}\nError accessing file at: {}",
                            filepath),
                    };

                    let mut headers = Headers::new();
                    headers.cache_control("no-cache");
                    headers.content_type(
                        "application/octet-stream"
                    );
                    headers.content_length(body.len());

                    let expect = Response {
                        status_line: StatusLine {
                            version: Version::OneDotOne,
                            status: Status($code)
                        },
                        headers,
                        body: if req.method() == Method::Head {
                            Body::Empty
                        } else {
                            body
                        }
                    };

                    let res = Response::for_route(&req.route(), &router);
                    assert_eq!(res, Ok(expect));
                )+
            }
        };
        ($label:ident:
            $(
                $method:ident $path:literal => $body:ident($inner:expr),
                $code:literal;
            )+
        ) => {
            #[test]
            fn $label() {
                let routes = BTreeMap::from([
                    $((Route::$method($path.into()), Target::$body($inner))),+
                ]);

                let router = Arc::new(Router(routes));

                $(
                    let body = Body::$body($inner.into());

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: body.clone()
                    };

                    let res = Response::for_route(&req.route(), &router);

                    let mut expect = Response {
                        status_line: StatusLine {
                            version: Version::OneDotOne,
                            status: Status($code)
                        },
                        headers: Headers::new(),
                        body
                    };

                    expect.headers.cache_control("no-cache");
                    expect.headers.content_length(expect.body.len());

                    match stringify!($label) {
                        s if s.eq_ignore_ascii_case("text_targets") => {
                            expect.headers
                                .content_type("text/plain; charset=utf-8");
                        },
                        s if s.eq_ignore_ascii_case("json_targets") => {
                            expect.headers
                                .content_type("application/json");
                        },
                        s if s.eq_ignore_ascii_case("xml_targets") => {
                            expect.headers
                                .content_type("application/xml");
                        },
                        _ => unreachable!(),
                    }

                    if Method::$method == Method::Head {
                        expect.body = Body::Empty;
                    }

                    assert_eq!(res, Ok(expect));
                )+
            }
        };
    }

    test_route_resolver! {
        empty_targets:
        Get "/empty1" => 200;
        Head "/empty2" => 200;
        Post "/empty3" => 201;
        Put "/empty4" => 200;
        Patch "/empty5" => 200;
        Delete "/empty6" => 200;
        Trace "/empty7" => 200;
        Options "/empty8" => 200;
        Connect "/empty9" => 200;
    }

    test_route_resolver! {
        text_targets:
        Get "/text1" => Text(b"test message 1"), 200;
        Head "/text2" => Text(b"test message 2"), 200;
    }

    test_route_resolver! {
        json_targets:
        Get "/json1" => Json(b"{\n\"data\": \"test data 1\"\n}"), 200;
        Head "/json2" => Json(b"{\n\"data\": \"test data 2\"\n}"), 200;
    }

    test_route_resolver! {
        xml_targets:
        Get "/xml1" => Xml(b"\
            <note>
            <to>Cat</to>
            <from>Dog</from>
            <heading>Woof</heading>
            <body>Who's a good boy?</body>
            </note>"
        ), 200;
        Head "/xml2" => Xml(b"\
            <note>
            <to>Dog</to>
            <from>Cat</from>
            <heading>Meow</heading>
            <body>Where's the mouse?</body>
            </note>"
        ), 200;
    }

    test_route_resolver! {
        html_targets:
        Get "/index" => "index.html", 200;
        Get "/about" => "about.html", 200;
        Head "/html" => "index.html", 200;
        Head "/about" => "about.html", 200;
    }

    test_route_resolver! {
        favicon_targets:
        Get "/favicon1" => "favicon.ico", 200;
        Head "/favicon2" => "favicon.ico", 200;
    }

    test_route_resolver! {
        file_targets:
        Get "/file1" => "test_file.dat", 200;
        Head "/file2" => "test_file.dat", 200;
    }
}

mod trait_impls {
    use super::*;

    macro_rules! trait_impl_test {
        ($label:ident implement $test_trait:ident: $( $test_type:ty ),+) => {
            #[test]
            const fn $label() {
                const fn trait_implementation_test<T: $test_trait>() {}
                $( trait_implementation_test::<$test_type>(); )+
            }
        };
    }

    trait_impl_test! [send_types implement Send:
        Body, Client, Connection, Header, HeaderKind, HeaderName, HeaderValue,
        Headers, Method, NetError, NetReader, NetResult<()>, NetWriter, NetParseError,
        Request, RequestLine, Response, Route, RouteBuilder, Router, Server,
        ServerBuilder<&str>, ServerConfig, ServerHandle<()>, Status,
        StatusLine, Target, ThreadPool, Version, Worker];

    trait_impl_test! [sync_types implement Sync:
        Body, Client, Connection, Header, HeaderKind, HeaderName, HeaderValue,
        Headers, Method, NetError, NetReader, NetResult<()>, NetWriter,
        NetParseError, Request, RequestLine, Response, Route, RouteBuilder,
        Router, Server, ServerBuilder<&str>, ServerConfig, ServerHandle<()>,
        Status, StatusLine, Target, ThreadPool, Version, Worker];

    trait_impl_test! [error_types implement Error:
        NetError, NetParseError];
}
