use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use crate::header::names::STANDARD_HEADERS;
use crate::util::trim_whitespace_bytes;
use crate::{
    Body, Client, Connection, Header, HeaderKind, HeaderName, HeaderValue,
    Headers, Method, NetError, NetParseError, NetReader, NetResult, NetWriter,
    Request, RequestLine, Response, Route, RouteBuilder, Router, Server,
    ServerBuilder, ServerConfig, ServerHandle, Status, StatusLine, Target,
    ThreadPool, Version, Worker,
};

macro_rules! test_parsing_from_str {
    (
        $target:ident $label:ident:
            $( $input:literal => $expected:expr; )+
            $( BAD_INPUT: $bad_input:literal; )+
    ) => {
        #[test]
        fn $label() {
            $( assert_eq!($input.parse::<$target>(), Ok($expected)); )+
            $( assert!($bad_input.parse::<$target>().is_err()); )+
        }
    };
}

macro_rules! test_parsing_from_int {
    (
        $target:ident $label:ident:
            $( $input:literal => $expected:expr; )+
            $( BAD_INPUT: $bad_input:literal; )+
    ) => {
        #[test]
        fn $label() {
            $( assert_eq!($target::try_from($input), Ok($expected)); )+
            $( assert!($target::try_from($bad_input).is_err()); )+
        }
    };
}

#[cfg(test)]
mod parse {
    use super::*;
    use self::{Method::*, Version::*};

    test_parsing_from_str! {
        Method method_from_str:
        "GET" => Get;
        "HEAD" => Head;
        "POST" => Post;
        "PUT" => Put;
        "PATCH" => Patch;
        "DELETE" => Delete;
        "TRACE" => Trace;
        "OPTIONS" => Options;
        "CONNECT" => Connect;
        BAD_INPUT: "FOO";
        BAD_INPUT: "get";
    }

    test_parsing_from_str! {
        Status status_from_str:
        "101 Switching Protocols" => Status(101);
        "201 Created" => Status(201);
        "300 Multiple Choices" => Status(300);
        "400 Bad Request" => Status(400);
        "501 Not Implemented" => Status(501);
        BAD_INPUT: "1234 Bad Status";
        BAD_INPUT: "abc";
    }

    test_parsing_from_int! {
        Status status_from_int:
        201_u16 => Status(201);
        202_u32 => Status(202);
        203_i32 => Status(203);
        BAD_INPUT: 1001_u16;
        BAD_INPUT: -123_i32;
        BAD_INPUT: 0_u16;
    }

    test_parsing_from_str! {
        Version version_from_str:
        "HTTP/0.9" => ZeroDotNine;
        "HTTP/1.0" => OneDotZero;
        "HTTP/1.1" => OneDotOne;
        "HTTP/2.0" => TwoDotZero;
        "HTTP/2" => TwoDotZero;
        "HTTP/3.0" => ThreeDotZero;
        "HTTP/3" => ThreeDotZero;
        BAD_INPUT: "HTTP/1.2";
        BAD_INPUT: "HTTP/1.10";
    }

    test_parsing_from_str! {
        RequestLine request_line_from_str:
        "GET /test HTTP/1.1\r\n" => RequestLine::new(Get, "/test");
        "HEAD /test HTTP/1.1\r\n" => RequestLine::new(Head, "/test");
        "POST /test HTTP/1.1\r\n" => RequestLine::new(Post, "/test");
        "PUT /test HTTP/1.1\r\n" => RequestLine::new(Put, "/test");
        "PATCH /test HTTP/1.1\r\n" => RequestLine::new(Patch, "/test");
        "DELETE /test HTTP/1.1\r\n" => RequestLine::new(Delete, "/test");
        "TRACE /test HTTP/1.1\r\n" => RequestLine::new(Trace, "/test");
        "OPTIONS /test HTTP/1.1\r\n" => RequestLine::new(Options, "/test");
        "CONNECT /test HTTP/1.1\r\n" => RequestLine::new(Connect, "/test");
        BAD_INPUT: "FOO bar baz";
        BAD_INPUT: "GET /test";
        BAD_INPUT: "GET";
    }

    test_parsing_from_str! {
        StatusLine status_line_from_str:
        "HTTP/1.1 100 Continue\r\n" => StatusLine::new(100);
        "HTTP/1.1 200 OK\r\n" => StatusLine::new(200);
        "HTTP/1.1 301 Moved Permanently\r\n" => StatusLine::new(301);
        "HTTP/1.1 403 Forbidden\r\n" => StatusLine::new(403);
        "HTTP/1.1 505 HTTP Version Not Supported\r\n" => StatusLine::new(505);
        BAD_INPUT: "HTTP/1.1";
        BAD_INPUT: "200 OK";
        BAD_INPUT: "FOO bar baz";
    }

    test_parsing_from_str! {
        Header header_from_str:
        "Accept: */*\r\n" =>
            Header::new("Accept", "*/*");
        "Host: rustnet/0.1\r\n" =>
            Header::new("Host", "rustnet/0.1");
        "Content-Length: 123\r\n" =>
            Header::new("Content-Length", "123");
        "Connection: keep-alive\r\n" =>
            Header::new("Connection", "keep-alive");
        "Content-Type: text/plain\r\n" =>
            Header::new("Content-Type", "text/plain");
        BAD_INPUT: "bad header";
    }

    #[test]
    fn standard_headers_from_str() {
        for &(std, lowercase) in STANDARD_HEADERS {
            let lower = String::from_utf8(lowercase.to_vec()).unwrap();
            let upper = lower.to_ascii_uppercase();
            let expected = HeaderName {
                inner: HeaderKind::Standard(std),
            };
            assert_eq!(HeaderName::from(lower.as_str()), expected.clone());
            assert_eq!(HeaderName::from(upper.as_str()), expected);
        }
    }

    #[test]
    fn uri_from_str() {
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

        let mut expected_hdrs = Headers::new();
        expected_hdrs.accept("*/*");
        expected_hdrs.connection("keep-alive");
        expected_hdrs.host("example.com");
        expected_hdrs.user_agent("xh/0.19.3");
        expected_hdrs.accept_encoding("gzip, deflate, br");
        expected_hdrs.header("Pineapple", "pizza");

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
mod router {
    use super::*;

    macro_rules! test_resolve_route {
        ($( $route:expr => $target:expr, $status:literal; )+) => {
            #[test]
            fn target_from_route() {
                let mut routes = Router::new();
                $( routes.mount($route, $target); )+
                routes.mount_shutdown_route();
                let router = Arc::new(routes);

                $(
                    let (target, status) = router.resolve(&$route);
                    assert_eq!(target, $target);
                    assert_eq!(status, $status);
                )+
            }
        };
    }

    test_resolve_route! {
        Route::Get("/empty1".into()) => Target::Empty, 200;
        Route::Head("/empty2".into()) => Target::Empty, 200;
        Route::Put("/text1".into()) => Target::Text("test1"), 200;
        Route::Patch("/text2".into()) => Target::Text("test2"), 200;
        Route::Post("/json1".into()) => 
            Target::Json("{\n\"data\": \"test data 1\"\n}"), 201;
        Route::Delete("/json2".into()) =>
            Target::Json("{\n\"data\": \"test data 2\"\n}"), 200;
        Route::Get("/xml1".into()) => Target::Xml("\
            <note>
            <to>Cat</to>
            <from>Dog</from>
            <heading>Woof</heading>
            <body>Who's a good boy?</body>
            </note>"
        ), 200;
        Route::Head("/xml2".into()) => Target::Xml("\
            <note>
            <to>Dog</to>
            <from>Cat</from>
            <heading>Meow</heading>
            <body>Where's the mouse?</body>
            </note>"
        ), 200;
        Route::Patch("/index".into()) => Target::Html("\
            <!DOCTYPE html>
            <html>
                <head>
                    <title>Home</title>
                </head>
                <body>
                    <p>Home page</p>
                </body>
            </html>"
        ), 200;
        Route::Post("/about".into()) => Target::Html("\
            <!DOCTYPE html>
            <html>
                <head>
                    <title>About</title>
                </head>
                <body>
                    <p>About page</p>
                </body>
            </html>"
        ), 201;
        Route::Delete("/bytes1".into()) => Target::Bytes(b"text bytes"), 200;
        Route::Put("/bytes2".into()) => Target::Bytes(b"text bytes"), 200;
        Route::Get("/favicon1".into()) =>
            Target::Favicon(Path::new("favicon.ico")), 200;
        Route::Head("/favicon2".into()) =>
            Target::Favicon(Path::new("favicon.ico")), 200;
        Route::Post("/file1".into()) => 
            Target::File(Path::new("test_file.dat")), 201;
        Route::Put("/file2".into()) =>
            Target::File(Path::new("test_file.dat")), 200;
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
        Headers, Method, NetError, NetReader, NetResult<()>, NetWriter,
        NetParseError, Request, RequestLine, Response, Route, RouteBuilder,
        Router, Server, ServerBuilder<&str>, ServerConfig, ServerHandle<()>,
        Status, StatusLine, Target, ThreadPool, Version, Worker];
    trait_impl_test! [sync_types implement Sync:
        Body, Client, Connection, Header, HeaderKind, HeaderName, HeaderValue,
        Headers, Method, NetError, NetReader, NetResult<()>, NetWriter,
        NetParseError, Request, RequestLine, Response, Route, RouteBuilder,
        Router, Server, ServerBuilder<&str>, ServerConfig, ServerHandle<()>,
        Status, StatusLine, Target, ThreadPool, Version, Worker];
    trait_impl_test! [error_types implement Error:
        NetError, NetParseError];
}
