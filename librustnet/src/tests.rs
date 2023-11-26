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
            let (name, value) = Header::parse(trim).unwrap();
            test_hdrs.insert(name, value);
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

// #[cfg(test)]
// mod router {
//     mod resolve {
//         use std::collections::BTreeMap;
//         use std::net::TcpStream;
//         use std::path::PathBuf;
//         use std::sync::Arc;
//         use crate::consts::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE};
//         use crate::{
//             Connection, Headers, Request, RequestLine, Response, Route, Router,
//             Status, StatusLine, Version, Method, Target,
//         };

//         macro_rules! test_empty_routes {
//             ($(
//                 $method:ident: $path:literal => Empty, $status:literal;
//             )+) => {
//                 #[test]
//                 fn empty_routes() {
//                     let routes = BTreeMap::from([
//                         $( (Route::new(Method::$method, $path), Target::Empty) ),+
//                     ]);

//                     let router = Arc::new(Router(routes));

//                     let localhost = TcpStream::connect("httpbin.org:80").unwrap();
//                     let dummy_connection = Connection::try_from(localhost).unwrap();

//                     $(
//                         let test_conn = dummy_connection.try_clone().unwrap();
//                         let expected_conn = dummy_connection.try_clone().unwrap();

//                         let req = Request {
//                             request_line: RequestLine {
//                                 method: Method::$method,
//                                 path: $path.to_string(),
//                                 version: Version::OneDotOne
//                             },
//                             headers: Headers::new(),
//                             body: None,
//                             conn: test_conn
//                         };

//                         let rtr = Arc::clone(&router);
//                         let res = Router::resolve(req, &rtr).unwrap();

//                         let mut headers = Headers::new();
//                         headers.insert(CACHE_CONTROL,
//                             "no-cache".as_bytes().into());

//                         let expected = Response {
//                             method: Method::$method,
//                             status_line: StatusLine {
//                                 version: Version::OneDotOne,
//                                 status: Status($status)
//                             },
//                             headers,
//                             body: None,
//                             conn: expected_conn
//                         };

//                         assert_eq!(res.method, expected.method);
//                         assert_eq!(res.status_line, expected.status_line);
//                         assert_eq!(res.headers, expected.headers);
//                         assert_eq!(res.body, expected.body);
//                     )+
//                 }
//             };
//         }

//         macro_rules! test_text_routes {
//             ($(
//                 $method:ident: $path:literal =>
//                 Text($text:literal), $status:literal;
//             )+) => {
//                 #[test]
//                 fn text_routes() {
//                     let routes = BTreeMap::from([
//                         $((Route::new(
//                                 Method::$method, $path),
//                                 Target::Text($text)
//                         )),+
//                     ]);

//                     let router = Arc::new(Router(routes));

//                     let localhost = TcpStream::connect("httpbin.org:80").unwrap();
//                     let dummy_connection = Connection::try_from(localhost).unwrap();

//                     $(
//                         let test_conn = dummy_connection.try_clone().unwrap();
//                         let expected_conn = dummy_connection.try_clone().unwrap();

//                         let req = Request {
//                             request_line: RequestLine {
//                                 method: Method::$method,
//                                 path: $path.to_string(),
//                                 version: Version::OneDotOne
//                             },
//                             headers: Headers::new(),
//                             body: None,
//                             conn: test_conn
//                         };

//                         let rtr = Arc::clone(&router);
//                         let res = Router::resolve(req, &rtr).unwrap();

//                         let mut headers = Headers::new();
//                         headers.insert(CACHE_CONTROL,
//                             "no-cache".as_bytes().into());
//                         headers.insert(CONTENT_LENGTH, "5".as_bytes().into());
//                         headers.insert(CONTENT_TYPE,
//                             "text/plain; charset=utf-8".as_bytes().into());

//                         let expected = Response {
//                             method: Method::$method,
//                             status_line: StatusLine {
//                                 version: Version::OneDotOne,
//                                 status: Status($status)
//                             },
//                             headers,
//                             body: Some(Vec::from($text)),
//                             conn: expected_conn
//                         };

//                         assert_eq!(res.method, expected.method);
//                         assert_eq!(res.status_line, expected.status_line);
//                         assert_eq!(res.headers, expected.headers);
//                         assert_eq!(res.body, expected.body);
//                     )+
//                 }
//             };
//         }

//         macro_rules! test_file_routes {
//             ($(
//                 $method:ident: $path:literal =>
//                 File($file:literal), $status:literal;
//             )+) => {
//                 #[test]
//                 fn file_routes() {
//                     let mut router = Router::new();
                    
//                     $( 
//                         let p: PathBuf = [
//                             "server",
//                             "static",
//                             $file
//                         ].iter().collect();
//                         let rt = Route::new(Method::$method, $path);
//                         router.mount(rt, Target::File(p));
//                     )+

//                     let router = Arc::new(Router::new());

//                     let localhost = TcpStream::connect("httpbin.org:80").unwrap();
//                     let dummy_connection = Connection::try_from(localhost).unwrap();

//                     $(
//                         let test_conn = dummy_connection.try_clone().unwrap();
//                         let expected_conn = dummy_connection.try_clone().unwrap();

//                         let req = Request {
//                             request_line: RequestLine {
//                                 method: Method::$method,
//                                 path: $path.to_string(),
//                                 version: Version::OneDotOne
//                             },
//                             headers: Headers::new(),
//                             body: None,
//                             conn: test_conn
//                         };

//                         let rtr = Arc::clone(&router);
//                         let res = Router::resolve(req, &rtr).unwrap();

//                         let mut headers = Headers::new();
//                         headers.insert(CACHE_CONTROL,
//                             "no-cache".as_bytes().into());
//                         headers.insert(CONTENT_LENGTH, "5".as_bytes().into());
//                         headers.insert(CONTENT_TYPE,
//                             "text/plain; charset=utf-8".as_bytes().into());

//                         let expected = Response {
//                             method: Method::$method,
//                             status_line: StatusLine {
//                                 version: Version::OneDotOne,
//                                 status: Status($status)
//                             },
//                             headers: Headers::new(),
//                             body: Some(Vec::from("text route test")),
//                             conn: expected_conn
//                         };

//                         assert_eq!(res.method, expected.method);
//                         assert_eq!(res.status_line, expected.status_line);
//                         assert_eq!(res.headers, expected.headers);
//                         assert_eq!(res.body, expected.body);
//                     )+
//                 }
//             };
//         }

//         test_empty_routes! {
//             Get: "/test1" => Empty, 200;
//             Head: "/test2" => Empty, 200;
//             Post: "/test3" => Empty, 200;
//             Put: "/test4" => Empty, 200;
//             Patch: "/test5" => Empty, 200;
//             Delete: "/test6" => Empty, 200;
//             Trace: "/test7" => Empty, 200;
//             Options: "/test8" => Empty, 200;
//             Connect: "127.0.0.1:1234" => Empty, 200;
//         }

//         test_text_routes! {
//             Get: "/test1" => Text("test1"), 200;
//             Head: "/test2" => Text("test2"), 200;
//             Post: "/test3" => Text("test3"), 200;
//             Put: "/test4" => Text("test4"), 200;
//             Patch: "/test5" => Text("test5"), 200;
//             Delete: "/test6" => Text("test6"), 200;
//             Trace: "/test7" => Text("test7"), 200;
//             Options: "/test8" => Text("test8"), 200;
//             Connect: "127.0.0.1:1234" => Text("test9"), 200;
//         }

//         test_file_routes! {
//             Get: "/test1" => File("index.html"), 200;
//             Head: "/test2" => File("index.html"), 200;
//             Post: "/test3" => File("index.html"), 200;
//             Put: "/test4" => File("index.html"), 200;
//             Patch: "/test5" => File("index.html"), 200;
//             Delete: "/test6" => File("index.html"), 200;
//             Trace: "/test7" => File("index.html"), 200;
//             Options: "/test8" => File("index.html"), 200;
//             Connect: "127.0.0.1:1234" => File("index.html"), 200;
//         }
//     }
// }