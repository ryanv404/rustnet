#[cfg(test)]
mod parse {
    use crate::{
        Client, Header, Headers, HeaderKind, HeaderName, HeaderValue,
        Method, RequestLine, Status, StatusLine, Version,
    };
    use crate::consts::{
        ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, TEST_HEADERS, USER_AGENT
    };
    use std::collections::BTreeMap;

    #[test]
    fn methods() {
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

    #[test]
    fn status_codes() {
        assert_eq!(
            "101 Switching Protocols".parse::<Status>(),
            Ok(Status(101))
        );
        assert_eq!("201 Created".parse::<Status>(), Ok(Status(201)));
        assert_eq!("300 Multiple Choices".parse::<Status>(), Ok(Status(300)));
        assert_eq!("400 Bad Request".parse::<Status>(), Ok(Status(400)));
        assert_eq!("501 Not Implemented".parse::<Status>(), Ok(Status(501)));
        assert!("abc".parse::<Status>().is_err());
    }

    #[test]
    fn versions() {
        assert_eq!("HTTP/0.9".parse::<Version>(), Ok(Version::ZeroDotNine));
        assert_eq!("HTTP/1.0".parse::<Version>(), Ok(Version::OneDotZero));
        assert_eq!("HTTP/1.1".parse::<Version>(), Ok(Version::OneDotOne));
        assert_eq!("HTTP/2.0".parse::<Version>(), Ok(Version::TwoDotZero));
        assert_eq!("HTTP/3.0".parse::<Version>(), Ok(Version::ThreeDotZero));
        assert!("HTTP/1.2".parse::<Version>().is_err());
    }

    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn request_lines() {
        macro_rules! parse_requestline {
            (SHOULD_ERR: $line:literal) => {
                let should_err = $line.parse::<RequestLine>();
                assert!(should_err.is_err());
            };
            ($method:ident: $line:literal) => {
                let req_line = $line.parse::<RequestLine>().unwrap();
                assert_eq!(req_line.method, Method::$method);
                assert_eq!(req_line.path, "/test".to_string());
                assert_eq!(req_line.version, Version::OneDotOne);
            };
        }

        parse_requestline!(Get: "GET /test HTTP/1.1\r\n");
        parse_requestline!(Head: "HEAD /test HTTP/1.1\r\n");
        parse_requestline!(Post: "POST /test HTTP/1.1\r\n");
        parse_requestline!(Put: "PUT /test HTTP/1.1\r\n");
        parse_requestline!(Patch: "PATCH /test HTTP/1.1\r\n");
        parse_requestline!(Delete: "DELETE /test HTTP/1.1\r\n");
        parse_requestline!(Trace: "TRACE /test HTTP/1.1\r\n");
        parse_requestline!(Options: "OPTIONS /test HTTP/1.1\r\n");
        parse_requestline!(Connect: "CONNECT /test HTTP/1.1\r\n");
        parse_requestline!(SHOULD_ERR: "GET");
        parse_requestline!(SHOULD_ERR: "GET /test");
        parse_requestline!(SHOULD_ERR: "FOO bar baz");
    }

    #[test]
    fn status_lines() {
        macro_rules! parse_statusline {
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

        parse_statusline!(100: "HTTP/1.1 100 Continue\r\n");
        parse_statusline!(200: "HTTP/1.1 200 OK\r\n");
        parse_statusline!(301: "HTTP/1.1 301 Moved Permanently\r\n");
        parse_statusline!(403: "HTTP/1.1 403 Forbidden\r\n");
        parse_statusline!(505: "HTTP/1.1 505 HTTP Version Not Supported\r\n");
        parse_statusline!(SHOULD_ERR: "HTTP/1.1");
        parse_statusline!(SHOULD_ERR: "200 OK");
        parse_statusline!(SHOULD_ERR: "FOO bar baz");
    }

    #[test]
    fn standard_headers() {
        for &(std_header, lowercase) in TEST_HEADERS {
            let lower = String::from_utf8(lowercase.to_vec()).unwrap();
            let upper = lower.to_ascii_uppercase();
            let expected = HeaderName { inner: HeaderKind::Standard(std_header) };
            assert_eq!(lower.parse::<HeaderName>(), Ok(expected.clone()));
            assert_eq!(upper.parse::<HeaderName>(), Ok(expected));
        }
    }

    #[test]
    fn custom_headers() {
        macro_rules! test_custom_headers {
            ($name:literal, $value:literal) =>  {{
                let test_name = $name.parse::<HeaderName>().unwrap();
                let expected_name = HeaderName {
                    inner: HeaderKind::Custom(Vec::from($name))
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

    #[test]
    fn headers_section() {
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
                b"pizza"[..].into()
            )
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

    #[test]
    fn uris() {
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
mod resolve_routes {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::{
        Body, Headers, Request, RequestLine, Response, Route, Router,
        Status, StatusLine, Version, Method, Target,
    };

    macro_rules! test_route_resolver {
        (empty: $( $method:ident $path:literal => $code:literal; )+) => {
            #[test]
            fn empty() {
                let routes = BTreeMap::from([
                    $( (Route::new(Method::$method, $path), Target::Empty) ),+
                ]);

                let router = Arc::new(Router(routes));

                $(
                    let body = Body::Empty;

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: body.clone(),
                        reader: None
                    };

                    let res = Response::from_route(&req.route(), &router);

                    let mut expect = Response {
                        status_line: StatusLine {
                            version: Version::OneDotOne,
                            status: Status($code)
                        },
                        headers: Headers::new(),
                        body,
                        writer: None
                    };

                    expect.headers.insert_cache_control("no-cache");

                    assert_eq!(res, expect);
                )+
            }
        };
        (html:
            $( $method:ident $path:literal => $file:literal, $code:literal; )+
        ) => {
            #[test]
            fn html() {
                $(
                    let mut routes = BTreeMap::new();

                    let filepath = concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../server/static/",
                        $file
                    );

                    routes.insert(
                        Route::new(Method::$method, $path),
                        Target::Html(PathBuf::from(filepath))
                    );

                    let router = Arc::new(Router(routes));

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: Body::Empty,
                        reader: None
                    };

                    let body = match fs::read_to_string(filepath) {
                        Ok(content) => Body::Html(content),
                        Err(e) => panic!(
                            "{e}\nError accessing HTML file at: {}",
                            filepath
                        ),
                    };

                    let mut headers = Headers::new();
                    headers.insert_cache_control("no-cache");
                    headers.insert_content_type(
                        "text/html; charset=utf-8"
                    );
                    headers.insert_content_length(body.len());

                    
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
                        },
                        writer: None
                    };

                    let res = Response::from_route(&req.route(), &router);

                    assert_eq!(res, expect);
                )+
            }
        };
        (favicon:
            $( $method:ident $path:literal => $file:literal, $code:literal; )+
        ) => {
            #[test]
            fn favicon() {
                $(
                    let mut routes = BTreeMap::new();

                    let filepath = concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../server/static/",
                        $file
                    );

                    routes.insert(
                        Route::new(Method::$method, $path),
                        Target::Favicon(PathBuf::from(filepath))
                    );

                    let router = Arc::new(Router(routes));

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: Body::Empty,
                        reader: None
                    };

                    let body = match fs::read(filepath) {
                        Ok(content) => Body::Favicon(content),
                        Err(e) => panic!(
                            "{e}\nError accessing HTML file at: {}",
                            filepath
                        ),
                    };

                    let mut headers = Headers::new();
                    headers.insert_cache_control("max-age=604800");
                    headers.insert_content_type("image/x-icon");
                    headers.insert_content_length(body.len());

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
                        },
                        writer: None
                    };

                    let res = Response::from_route(&req.route(), &router);

                    assert_eq!(res, expect);
                )+
            }
        };
        (file:
            $( $method:ident $path:literal => $file:literal, $code:literal; )+
        ) => {
            #[test]
            fn file() {
                $(
                    let mut routes = BTreeMap::new();

                    let filepath = concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../server/static/",
                        $file
                    );

                    routes.insert(
                        Route::new(Method::$method, $path),
                        Target::File(PathBuf::from(filepath))
                    );

                    let router = Arc::new(Router(routes));

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: Body::Empty,
                        reader: None
                    };

                    let body = match fs::read(filepath) {
                        Ok(content) => Body::Bytes(content),
                        Err(e) => panic!(
                            "{e}\nError accessing HTML file at: {}",
                            filepath
                        ),
                    };

                    let mut headers = Headers::new();
                    headers.insert_cache_control("no-cache");
                    headers.insert_content_type(
                        "application/octet-stream"
                    );
                    headers.insert_content_length(body.len());

                    
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
                        },
                        writer: None
                    };

                    let res = Response::from_route(&req.route(), &router);

                    assert_eq!(res, expect);
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
                    $((Route::new(Method::$method, $path), Target::$body($inner))),+
                ]);

                let router = Arc::new(Router(routes));

                $(
                    let body = Body::$body(String::from($inner));

                    let req = Request {
                        request_line: RequestLine {
                            method: Method::$method,
                            path: $path.to_string(),
                            version: Version::OneDotOne
                        },
                        headers: Headers::new(),
                        body: body.clone(),
                        reader: None
                    };

                    let res = Response::from_route(&req.route(), &router);

                    let mut expect = Response {
                        status_line: StatusLine {
                            version: Version::OneDotOne,
                            status: Status($code)
                        },
                        headers: Headers::new(),
                        body,
                        writer: None
                    };

                    expect.headers.insert_cache_control("no-cache");
                    expect.headers.insert_content_length(expect.body.len());

                    match stringify!($label) {
                        s if s.eq_ignore_ascii_case("text") => {
                            expect.headers
                                .insert_content_type("text/plain; charset=utf-8");
                        },
                        s if s.eq_ignore_ascii_case("json") => {
                            expect.headers
                                .insert_content_type("application/json");
                        },
                        s if s.eq_ignore_ascii_case("xml") => {
                            expect.headers
                                .insert_content_type("application/xml");
                        },
                        _ => unreachable!(),
                    }

                    if Method::$method == Method::Head {
                        expect.body = Body::Empty;
                    }

                    assert_eq!(res, expect);
                )+
            }
        };
    }

    test_route_resolver! {
        empty:
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
        text:
        Get "/text1" => Text("test message 1"), 200;
        Head "/text2" => Text("test message 2"), 200;
    }

    test_route_resolver! {
        json:
        Get "/json1" => Json("{\n\"data\": \"test data 1\"\n}"), 200;
        Head "/json2" => Json("{\n\"data\": \"test data 2\"\n}"), 200;
    }

    test_route_resolver! {
        xml:
        Get "/xml1" => Xml("\
            <note>
            <to>Cat</to>
            <from>Dog</from>
            <heading>Woof</heading>
            <body>Who's a good boy?</body>
            </note>"
        ), 200;
        Head "/xml2" => Xml("\
            <note>
            <to>Dog</to>
            <from>Cat</from>
            <heading>Meow</heading>
            <body>Where's the mouse?</body>
            </note>"
        ), 200;
    }

    test_route_resolver! {
        html:
        Get "/index" => "index.html", 200;
        Get "/about" => "about.html", 200;
        Head "/html" => "index.html", 200;
        Head "/about" => "about.html", 200;
    }

    test_route_resolver! {
        favicon:
        Get "/file1" => "favicon.ico", 200;
        Head "/file2" => "favicon.ico", 200;
    }

    test_route_resolver! {
        file:
        Get "/file1" => "test_file.dat", 200;
        Head "/file2" => "test_file.dat", 200;
    }
}

#[cfg(test)]
mod send_sync {
    use crate::{
        Body, Client, ClientBuilder, Header, Headers, HeaderKind, HeaderName,
        HeaderValue, Method, NetError, NetReader, NetResult, NetWriter,
        ParseErrorKind, Request, RequestLine, Response, Route, RouteBuilder,
        Router, Server, ServerBuilder, Status, StatusLine, Target, Task,
        ThreadPool, Version, Worker,
    };

    #[test]
    const fn send_tests() {
        const fn type_is_send<T: Send>() {}
        type_is_send::<Body>();
        type_is_send::<Client>();
        type_is_send::<ClientBuilder<&str>>();
        type_is_send::<Header>();
        type_is_send::<Headers>();
        type_is_send::<HeaderKind>();
        type_is_send::<HeaderName>();
        type_is_send::<HeaderValue>();
        type_is_send::<Method>();
        type_is_send::<NetError>();
        type_is_send::<NetReader>();
        type_is_send::<NetResult<()>>();
        type_is_send::<NetWriter>();
        type_is_send::<ParseErrorKind>();
        type_is_send::<Request>();
        type_is_send::<RequestLine>();
        type_is_send::<Response>();
        type_is_send::<Route>();
        type_is_send::<RouteBuilder>();
        type_is_send::<Router>();
        type_is_send::<Server>();
        type_is_send::<ServerBuilder<&str>>();
        type_is_send::<Status>();
        type_is_send::<StatusLine>();
        type_is_send::<Target>();
        type_is_send::<Task>();
        type_is_send::<ThreadPool>();
        type_is_send::<Version>();
        type_is_send::<Worker>();
    }

    #[test]
    const fn sync_tests() {
        const fn type_is_sync<T: Sync>() {}
        type_is_sync::<Body>();
        type_is_sync::<Client>();
        type_is_sync::<ClientBuilder<&str>>();
        type_is_sync::<Header>();
        type_is_sync::<Headers>();
        type_is_sync::<HeaderKind>();
        type_is_sync::<HeaderName>();
        type_is_sync::<HeaderValue>();
        type_is_sync::<Method>();
        type_is_sync::<NetError>();
        type_is_sync::<NetReader>();
        type_is_sync::<NetResult<()>>();
        type_is_sync::<NetWriter>();
        type_is_sync::<ParseErrorKind>();
        type_is_sync::<Request>();
        type_is_sync::<RequestLine>();
        type_is_sync::<Response>();
        type_is_sync::<Route>();
        type_is_sync::<Router>();
        type_is_sync::<RouteBuilder>();
        type_is_sync::<Server>();
        type_is_sync::<ServerBuilder<&str>>();
        type_is_sync::<Status>();
        type_is_sync::<StatusLine>();
        type_is_sync::<Target>();
        type_is_sync::<ThreadPool>();
        type_is_sync::<Version>();
        type_is_sync::<Worker>();
    }
}
