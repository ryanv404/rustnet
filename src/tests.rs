use std::error::Error;
use std::str::{self, FromStr};

use crate::header::names::STANDARD_HEADERS;
use crate::header_name::{
    ACCEPT, CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, HOST,
};
use crate::util::{self, trim_whitespace_bytes};
use crate::{
    Body, Client, Connection, Header, HeaderName, HeaderNameInner,
    HeaderValue, Headers, Method, NetError, NetParseError, NetResult,
    Request, RequestLine, Response, Route, RouteBuilder, Router, Server,
    ServerBuilder, ServerConfig, ServerHandle, Status, StatusCode,
    StatusLine, Target, ThreadPool, Version, Worker,
};

macro_rules! test_parsing_from_str {
    (
        $target:ident $label:ident:
            $( $input:literal => $expected:expr; )+
            $( BAD_INPUT: $bad_input:literal; )*
    ) => {
        #[test]
        fn $label() {
            $( assert_eq!($target::from_str($input), Ok($expected)); )+
            $( assert!($target::from_str($bad_input).is_err()); )*
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

    test_parsing_from_str! {
        Method method_from_str:
        "GET" => Method::Get;
        "HEAD" => Method::Head;
        "POST" => Method::Post;
        "PUT" => Method::Put;
        "PATCH" => Method::Patch;
        "DELETE" => Method::Delete;
        "TRACE" => Method::Trace;
        "OPTIONS" => Method::Options;
        "CONNECT" => Method::Connect;
        "SHUTDOWN" => Method::Custom("SHUTDOWN".to_string());
        "Foo" => Method::Custom("Foo".to_string());
        "get" => Method::Custom("get".to_string());
    }

    test_parsing_from_str! {
        StatusCode status_code_from_str:
        "101" => StatusCode(101u16);
        "201" => StatusCode(201u16);
        "300" => StatusCode(300u16);
        "400" => StatusCode(400u16);
        "501" => StatusCode(501u16);
        BAD_INPUT: "1234";
        BAD_INPUT: "abc";
        BAD_INPUT: "-12";
    }

    test_parsing_from_int! {
        StatusCode status_code_from_int:
        201_u16 => StatusCode(201u16);
        202_u32 => StatusCode(202u16);
        203_i32 => StatusCode(203u16);
        BAD_INPUT: 1001_u16;
        BAD_INPUT: -123_i32;
        BAD_INPUT: 0_u16;
    }

    test_parsing_from_str! {
        Status status_from_str:
        "101 Switching Protocols" => Status(StatusCode(101u16));
        "201 Created" => Status(StatusCode(201u16));
        "300 Multiple Choices" => Status(StatusCode(300u16));
        "400 Bad Request" => Status(StatusCode(400u16));
        "501 Not Implemented" => Status(StatusCode(501u16));
        BAD_INPUT: "1234 Bad Status";
        BAD_INPUT: "abc";
    }

    test_parsing_from_int! {
        Status status_from_int:
        201_u16 => Status(StatusCode(201u16));
        202_u32 => Status(StatusCode(202u16));
        203_i32 => Status(StatusCode(203u16));
        BAD_INPUT: 1001_u16;
        BAD_INPUT: -123_i32;
        BAD_INPUT: 0_u16;
    }

    test_parsing_from_str! {
        Version version_from_str:
        "HTTP/0.9" => Version::ZeroDotNine;
        "HTTP/1.0" => Version::OneDotZero;
        "HTTP/1.1" => Version::OneDotOne;
        "HTTP/2.0" => Version::TwoDotZero;
        "HTTP/2" => Version::TwoDotZero;
        "HTTP/3.0" => Version::ThreeDotZero;
        "HTTP/3" => Version::ThreeDotZero;
        BAD_INPUT: "HTTP/1.2";
        BAD_INPUT: "HTTP/1.10";
    }

    test_parsing_from_str! {
        RequestLine request_line_from_str:
        "GET /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Get, "/test");
        "HEAD /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Head, "/test");
        "POST /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Post, "/test");
        "PUT /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Put, "/test");
        "PATCH /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Patch, "/test");
        "DELETE /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Delete, "/test");
        "TRACE /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Trace, "/test");
        "OPTIONS /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Options, "/test");
        "CONNECT /test HTTP/1.1\r\n" =>
            RequestLine::new(&Method::Connect, "/test");
        BAD_INPUT: "FOO bar baz";
        BAD_INPUT: "GET /test";
        BAD_INPUT: "GET";
    }

    test_parsing_from_str! {
        StatusLine status_line_from_str:
        "HTTP/1.1 100 Continue\r\n" =>
            StatusLine::from(StatusCode(100u16));
        "HTTP/1.1 200 OK\r\n" =>
            StatusLine::from(StatusCode(200u16));
        "HTTP/1.1 301 Moved Permanently\r\n" =>
            StatusLine::from(StatusCode(301u16));
        "HTTP/1.1 403 Forbidden\r\n" =>
            StatusLine::from(StatusCode(403u16));
        "HTTP/1.1 505 HTTP Version Not Supported\r\n" =>
            StatusLine::from(StatusCode(505u16));
        BAD_INPUT: "HTTP/1.1";
        BAD_INPUT: "200 OK";
        BAD_INPUT: "FOO bar baz";
    }

    test_parsing_from_str! {
        Header header_from_str:
        "Accept: */*\r\n" =>
            Header { name: ACCEPT, value: "*/*".into() };
        "Host: rustnet/0.1\r\n" =>
            Header { name: HOST, value: "rustnet/0.1".into() };
        "Content-Length: 123\r\n" =>
            Header { name: CONTENT_LENGTH, value: "123".into() };
        "Connection: keep-alive\r\n" =>
            Header { name: CONNECTION, value: "keep-alive".into() };
        "Content-Type: text/plain\r\n" =>
            Header { name: CONTENT_TYPE, value: "text/plain".into() };
        BAD_INPUT: "bad header";
    }

    #[test]
    fn standard_headers_from_str() {
        for &(std, lowercase) in STANDARD_HEADERS {
            let lower = str::from_utf8(lowercase).unwrap();
            let upper = lower.to_ascii_uppercase();
            let expected = HeaderName {
                inner: HeaderNameInner::Standard(std),
            };
            assert_eq!(HeaderName::from(lower), expected.clone());
            assert_eq!(HeaderName::from(upper.as_str()), expected);
        }
    }

    #[test]
    fn uri_from_str() {
        macro_rules! test_uri_parser {
            ( $(SHOULD_ERROR: $uri:literal;)+ ) => {{
                $(
                    let parse_result = util::parse_uri($uri);
                    assert!(parse_result.is_err());
                )+
            }};
            ( $($uri:literal: $addr:literal, $path:literal;)+ ) => {{
                $(
                    let (test_addr, test_path) = util::parse_uri($uri).unwrap();
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
        let headers_str = "\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_hdrs = Headers::from_str(headers_str).unwrap();

        let mut expected_hdrs = Headers::new();
        expected_hdrs.accept("*/*");
        expected_hdrs.user_agent("xh/0.19.3");
        expected_hdrs.connection("keep-alive");
        expected_hdrs.header("Pineapple", "pizza");
        expected_hdrs.header("Host", "example.com");
        expected_hdrs.accept_encoding("gzip, deflate, br");

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
        Body, Client, Connection, Header, HeaderNameInner, HeaderName,
        HeaderValue, Headers, Method, NetError, NetResult<()>,
        NetParseError, Request, RequestLine, Response, Route, RouteBuilder,
        Router, Server, ServerBuilder<&str>, ServerConfig, ServerHandle<()>,
        Status, StatusCode, StatusLine, Target, ThreadPool, Version, Worker];
    trait_impl_test! [sync_types implement Sync:
        Body, Client, Connection, Header, HeaderNameInner, HeaderName,
        HeaderValue, Headers, Method, NetError, NetResult<()>,
        NetParseError, Request, RequestLine, Response, Route, RouteBuilder,
        Router, Server, ServerBuilder<&str>, ServerConfig, ServerHandle<()>,
        Status, StatusCode, StatusLine, Target, ThreadPool, Version, Worker];
    trait_impl_test! [error_types implement Error:
        NetError, NetParseError];
}
