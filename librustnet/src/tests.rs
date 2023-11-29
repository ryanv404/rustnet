#[cfg(test)]
mod parse {
    use crate::{
        Client, Header, Headers, HeaderKind, HeaderName, HeaderValue,
        Method, RequestLine, Status, Version,
    };
    use crate::consts::{
        ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, TEST_HEADERS, USER_AGENT
    };
    use std::collections::BTreeMap;

    #[test]
    fn methods() {
        assert_eq!(Method::parse(Some("GET")), Ok(Method::Get));
        assert_eq!(Method::parse(Some("HEAD")), Ok(Method::Head));
        assert_eq!(Method::parse(Some("POST")), Ok(Method::Post));
        assert_eq!(Method::parse(Some("PUT")), Ok(Method::Put));
        assert_eq!(Method::parse(Some("PATCH")), Ok(Method::Patch));
        assert_eq!(Method::parse(Some("DELETE")), Ok(Method::Delete));
        assert_eq!(Method::parse(Some("TRACE")), Ok(Method::Trace));
        assert_eq!(Method::parse(Some("OPTIONS")), Ok(Method::Options));
        assert_eq!(Method::parse(Some("CONNECT")), Ok(Method::Connect));
        assert!(Method::parse(Some("get")).is_err());
        assert!(Method::parse(Some("FOO")).is_err());
    }

    #[test]
    fn status_codes() {
        assert_eq!(Status::parse(Some("100")), Ok(Status(100)));
        assert_eq!(Status::parse(Some("201")), Ok(Status(201)));
        assert_eq!(Status::parse(Some("301")), Ok(Status(301)));
        assert_eq!(Status::parse(Some("403")), Ok(Status(403)));
        assert_eq!(Status::parse(Some("500")), Ok(Status(500)));
        assert!(Status::parse(Some("abc")).is_err());
    }

    #[test]
    fn versions() {
        assert_eq!(Version::parse(Some("HTTP/0.9")), Ok(Version::ZeroDotNine));
        assert_eq!(Version::parse(Some("HTTP/1.0")), Ok(Version::OneDotZero));
        assert_eq!(Version::parse(Some("HTTP/1.1")), Ok(Version::OneDotOne));
        assert_eq!(Version::parse(Some("HTTP/2.0")), Ok(Version::TwoDotZero));
        assert_eq!(Version::parse(Some("HTTP/3.0")), Ok(Version::ThreeDotZero));
        assert!(Version::parse(Some("HTTP/1.2")).is_err());
    }

    #[test]
    fn request_lines() {
        macro_rules! parse_reqline {
            (SHOULD_ERR: $line:literal) => {
                let should_err = RequestLine::parse($line);
                assert!(should_err.is_err());
            };
            ($method:ident: $line:literal) => {
                let req_line = RequestLine::parse($line).unwrap();
                assert_eq!(req_line.method, Method::$method);
                assert_eq!(req_line.path, "/test".to_string());
                assert_eq!(req_line.version, Version::OneDotOne);
            };
        }

        parse_reqline!(Get: "GET /test HTTP/1.1\r\n");
        parse_reqline!(Head: "HEAD /test HTTP/1.1\r\n");
        parse_reqline!(Post: "POST /test HTTP/1.1\r\n");
        parse_reqline!(Put: "PUT /test HTTP/1.1\r\n");
        parse_reqline!(Patch: "PATCH /test HTTP/1.1\r\n");
        parse_reqline!(Delete: "DELETE /test HTTP/1.1\r\n");
        parse_reqline!(Trace: "TRACE /test HTTP/1.1\r\n");
        parse_reqline!(Options: "OPTIONS /test HTTP/1.1\r\n");
        parse_reqline!(Connect: "CONNECT /test HTTP/1.1\r\n");
        parse_reqline!(SHOULD_ERR: "GET");
        parse_reqline!(SHOULD_ERR: "GET /test");
        parse_reqline!(SHOULD_ERR: "FOO bar baz");
    }

    #[test]
    fn standard_headers() {
        for &(std_header, lowercase) in TEST_HEADERS {
            let lower = String::from_utf8(lowercase.to_vec()).unwrap();
            let upper = lower.to_ascii_uppercase();
            let expected = HeaderName { inner: HeaderKind::Standard(std_header) };
            assert_eq!(HeaderName::parse(Some(&lower)), Ok(expected.clone()));
            assert_eq!(HeaderName::parse(Some(&upper)), Ok(expected));
        }
    }

    #[test]
    fn custom_headers() {
        macro_rules! test_custom_headers {
            ($name:literal, $value:literal) =>  {{
                let test_name = HeaderName::parse(Some($name));
                let expected_name = HeaderName {
                    inner: HeaderKind::Custom(Vec::from($name))
                };
                let test_value = HeaderValue::parse(Some($value));
                let expected_value = HeaderValue(Vec::from($value));
                assert_eq!(test_name, Ok(expected_name));
                assert_eq!(test_value, Ok(expected_value));
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
            (ACCEPT, "*/*".as_bytes().into()),
            (ACCEPT_ENCODING, "gzip, deflate, br".as_bytes().into()),
            (CONNECTION, "keep-alive".as_bytes().into()),
            (HOST, "example.com".as_bytes().into()),
            (USER_AGENT, "xh/0.19.3".as_bytes().into()),
            (HeaderName {
                inner: HeaderKind::Custom(Vec::from("Pineapple")),
            }, "pizza".as_bytes().into()),
        ]));

        let mut test_hdrs = Headers::new();

        for line in headers_section.split('\n') {
            let trim = line.trim();
            if trim.is_empty() { break; }
            let header = Header::parse(trim).unwrap();
            test_hdrs.insert(header.name, header.value);
        }

        // Compare total lengths.
        assert_eq!(test_hdrs.0.len(), expected_hdrs.0.len());

        // Compare each header name and header value.
        test_hdrs.0.iter().zip(expected_hdrs.0.iter()).for_each(
            |((test_name, test_value), (exp_name, exp_value))| {
                assert_eq!(*test_name, *exp_name);
                assert_eq!(*test_value, *exp_value); 
            });
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
mod router {
    mod resolve {
        use std::collections::BTreeMap;
        use std::sync::Arc;

        use crate::{
            Body, Headers, Request, RequestLine, Response, Route, Router,
            Status, StatusLine, Version, Method, Target,
        };

        macro_rules! test_empty_routes {
            (
                $label:ident:
                $(
                    $method:ident $path:literal => $body:ident, $code:literal;
                )+
            ) => {
                #[test]
                fn $label() {
                    let routes = BTreeMap::from([
                        $( (Route::new(Method::$method, $path), Target::$body) ),+
                    ]);

                    let router = Arc::new(Router(routes));

                    $(
                        let body = Body::$body;

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

                        let res = Response::from_route(&req.route(), &router).unwrap();

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

                        assert_eq!(res.status_line, expect.status_line);
                        assert_eq!(res.headers, expect.headers);
                        assert_eq!(res.body, expect.body);
                    )+
                }
            };
        }

        macro_rules! test_routes {
            (
                $label:ident:
                $(
                    $method:ident $path:literal =>
                    $body:ident($inner:expr), $code:literal;
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

                        let res = Response::from_route(&req.route(), &router).unwrap();

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
                            s if s.eq_ignore_ascii_case("text_routes") => {
                                expect.headers.insert_content_type("text/plain; charset=utf-8");
                            },
                            s if s.eq_ignore_ascii_case("html_routes") => {
                                expect.headers.insert_content_type("text/html; charset=utf-8");
                            },
                            s if s.eq_ignore_ascii_case("json_routes") => {
                                expect.headers.insert_content_type("application/json");
                            },
                            s if s.eq_ignore_ascii_case("xml_routes") => {
                                expect.headers.insert_content_type("application/xml");
                            },
                            s if s.eq_ignore_ascii_case("bytes_routes") => {
                                expect.headers.insert_content_type("application/octet-stream");
                            },
                            _ => unreachable!(),
                        }

                        if Method::$method == Method::Head {
                            expect.body = Body::Empty;
                        }

                        assert_eq!(res.status_line, expect.status_line);
                        assert_eq!(res.headers, expect.headers);
                        assert_eq!(res.body, expect.body);
                    )+
                }
            };
        }

        test_empty_routes! {
            empty_routes:
            Get "/empty1" => Empty, 200;
            Head "/empty2" => Empty, 200;
            Post "/empty3" => Empty, 201;
            Put "/empty4" => Empty, 200;
            Patch "/empty5" => Empty, 200;
            Delete "/empty6" => Empty, 200;
            Trace "/empty7" => Empty, 200;
            Options "/empty8" => Empty, 200;
            Connect "/empty9" => Empty, 200;
        }

        test_routes! {
            text_routes:
            Get "/text1" => Text("text1"), 200;
            Head "/text2" => Text("text2"), 200;
            Post "/text3" => Text("text3"), 201;
            Put "/text4" => Text("text4"), 200;
            Patch "/text5" => Text("text5"), 200;
            Delete "/text6" => Text("text6"), 200;
            Trace "/text7" => Text("text7"), 200;
            Options "/text8" => Text("text8"), 200;
            Connect "/text9" => Text("text9"), 200;
        }

        test_routes! {
            json_routes:
            Get "/json1" => Json("json1"), 200;
            Head "/json2" => Json("json2"), 200;
            Post "/json3" => Json("json3"), 201;
            Put "/json4" => Json("json4"), 200;
            Patch "/json5" => Json("json5"), 200;
            Delete "/json6" => Json("json6"), 200;
            Trace "/json7" => Json("json7"), 200;
            Options "/json8" => Json("json8"), 200;
            Connect "/json9" => Json("json9"), 200;
        }

        // test_routes! {
        //     html_routes:
        //     Get "/html1" => Html("html1"), 200;
        //     Head "/html2" => Html("html2"), 200;
        //     Post "/html3" => Html("html3"), 201;
        //     Put "/html4" => Html("html4"), 200;
        //     Patch "/html5" => Html("html5"), 200;
        //     Delete "/html6" => Html("html6"), 200;
        //     Trace "/html7" => Html("html7"), 200;
        //     Options "/html8" => Html("html8"), 200;
        //     Connect "/html9" => Html("html9"), 200;
        // }

        test_routes! {
            xml_routes:
            Get "/xml1" => Xml("xml1"), 200;
            Head "/xml2" => Xml("xml2"), 200;
            Post "/xml3" => Xml("xml3"), 201;
            Put "/xml4" => Xml("xml4"), 200;
            Patch "/xml5" => Xml("xml5"), 200;
            Delete "/xml6" => Xml("xml6"), 200;
            Trace "/xml7" => Xml("xml7"), 200;
            Options "/xml8" => Xml("xml8"), 200;
            Connect "/xml9" => Xml("xml9"), 200;
        }
    }
}

#[cfg(test)]
mod send_sync {
    use crate::{
        Body, Client, Header, Headers, HeaderKind, HeaderName,
        HeaderValue, Method, NetReader, NetWriter, Request, RequestLine,
        Response, Route, RouteBuilder, Router, Server, Status, StatusLine,
        Target, Version,
    };

    #[test]
    fn send_tests() {
        fn type_is_send<T: Send>() {}
        type_is_send::<Body>();
        type_is_send::<Client>();
        type_is_send::<Header>();
        type_is_send::<HeaderKind>();
        type_is_send::<HeaderName>();
        type_is_send::<HeaderValue>();
        type_is_send::<Headers>();
        type_is_send::<Method>();
        type_is_send::<NetReader>();
        type_is_send::<NetWriter>();
        type_is_send::<Request>();
        type_is_send::<RequestLine>();
        type_is_send::<Response>();
        type_is_send::<Route>();
        type_is_send::<RouteBuilder>();
        type_is_send::<Router>();
        type_is_send::<Server>();
        type_is_send::<Status>();
        type_is_send::<StatusLine>();
        type_is_send::<Target>();
        type_is_send::<Version>();
    }

    #[test]
    fn sync_tests() {
        fn type_is_sync<T: Sync>() {}
        type_is_sync::<Body>();
        type_is_sync::<Client>();
        type_is_sync::<Header>();
        type_is_sync::<HeaderKind>();
        type_is_sync::<HeaderName>();
        type_is_sync::<HeaderValue>();
        type_is_sync::<Headers>();
        type_is_sync::<Method>();
        type_is_sync::<NetReader>();
        type_is_sync::<NetWriter>();
        type_is_sync::<Request>();
        type_is_sync::<RequestLine>();
        type_is_sync::<Response>();
        type_is_sync::<Route>();
        type_is_sync::<Router>();
        type_is_sync::<RouteBuilder>();
        type_is_sync::<Server>();
        type_is_sync::<Status>();
        type_is_sync::<StatusLine>();
        type_is_sync::<Target>();
        type_is_sync::<Version>();
    }
}
