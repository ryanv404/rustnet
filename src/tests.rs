use std::str::{self, FromStr};

use crate::{
    Body, Client, Connection, Header, Headers, Method, NetError, NetParseError,
    Request, Response, Route, Router, Server, ServerHandle, Status, Target,
    UriPath, Version, DEFAULT_NAME,
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
mod methods {
    use super::*;

    test_parsing_from_str! {
        Method from_str:
        "ANY" => Method::Any;
        "GET" => Method::Get;
        "HEAD" => Method::Head;
        "POST" => Method::Post;
        "PUT" => Method::Put;
        "PATCH" => Method::Patch;
        "DELETE" => Method::Delete;
        "TRACE" => Method::Trace;
        "OPTIONS" => Method::Options;
        "CONNECT" => Method::Connect;
        "SHUTDOWN" => Method::Shutdown;
        BAD_INPUT: "Foo";
        BAD_INPUT: "get";
    }
}

#[cfg(test)]
mod statuses {
    use super::*;

    test_parsing_from_str! {
        Status from_str:
        "101 Switching Protocols" => Status(101_u16);
        "201 Created" => Status(201_u16);
        "300 Multiple Choices" => Status(300_u16);
        "400 Bad Request" => Status(400_u16);
        "501 Not Implemented" => Status(501_u16);
        BAD_INPUT: "1234 Bad Status";
        BAD_INPUT: "abc";
    }

    test_parsing_from_int! {
        Status from_int:
        201_u16 => Status(201_u16);
        202_u16 => Status(202_u16);
        203_u16 => Status(203_u16);
        BAD_INPUT: 1001_u16;
        BAD_INPUT: 99_u16;
        BAD_INPUT: 0_u16;
    }
}

#[cfg(test)]
mod versions {
    use super::*;

    test_parsing_from_str! {
        Version from_str:
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
}

#[cfg(test)]
mod standard_headers {
    use super::*;
    use crate::HeaderName;
    use crate::headers::names::{HeaderNameInner, STANDARD_HEADERS};

    #[test]
    fn from_str() {
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
}

#[cfg(test)]
mod single_header {
    use super::*;
    use crate::headers::names::{
        ACCEPT, CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, HOST,
    };

    test_parsing_from_str! {
        Header from_str:
        "Accept: */*\r\n" =>
            Header { name: ACCEPT, value: "*/*".into() };
        "Host: rustnet/0.1.1\r\n" =>
            Header { name: HOST, value: DEFAULT_NAME.into() };
        "Content-Length: 123\r\n" =>
            Header { name: CONTENT_LENGTH, value: "123".into() };
        "Connection: keep-alive\r\n" =>
            Header { name: CONNECTION, value: "keep-alive".into() };
        "Content-Type: text/plain\r\n" =>
            Header { name: CONTENT_TYPE, value: "text/plain".into() };
        BAD_INPUT: "bad header";
    }
}

#[cfg(test)]
mod many_headers {
    use super::*;
    use crate::headers::names::{HOST, USER_AGENT};

    #[test]
    fn from_str() {
        let test_headers = Headers::from_str("\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n"
        )
        .unwrap();

        let mut expected_headers = Headers::new();
        expected_headers.add_accept("*/*");
        expected_headers.add_connection("keep-alive");
        expected_headers.insert(HOST, "example.com".into());
        expected_headers.insert(USER_AGENT, "xh/0.19.3".into());
        expected_headers.add_accept_encoding("gzip, deflate, br");
        expected_headers.insert("Pineapple".into(), "pizza".into());

        assert_eq!(test_headers, expected_headers);
    }
}

#[cfg(test)]
mod uris {
    #[test]
    fn from_str() {
        macro_rules! test_uri_parser {
            ( $(SHOULD_ERROR: $uri:literal;)+ ) => {{
                $(
                    let parse_result = $crate::utils::parse_uri($uri);
                    assert!(parse_result.is_err());
                )+
            }};
            ( $($uri:literal: $addr:literal, $path:literal;)+ ) => {{
                $(
                    let (test_addr, test_path) = $crate::utils::parse_uri($uri)
                        .unwrap();
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
mod requests {
    use super::*;
    use crate::headers::names::{HOST, USER_AGENT};

    #[test]
    fn from_bytes() {
        let input = b"\
            GET /test HTTP/1.1\r\n\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Content-Length: 0\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_req = Request::try_from(&input[..]).unwrap();

        let mut headers = Headers::new();
        headers.add_accept("*/*");
        headers.add_content_length(0);
        headers.add_connection("keep-alive");
        headers.add_accept_encoding("gzip, deflate, br");
        headers.insert(HOST, "example.com".into());
        headers.insert(USER_AGENT, "xh/0.19.3".into());
        headers.insert("Pineapple".into(), "pizza".into());

        let expected_req = Request {
            path: UriPath("/test".into()),
            headers,
            ..Request::default()
        };

        assert_eq!(test_req, expected_req);
    }
}

#[cfg(test)]
mod responses {
    use super::*;
    use crate::headers::names::SERVER;

    #[test]
    fn from_bytes() {
        let input = b"\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 0\r\n\
            Connection: keep-alive\r\n\
            Server: example.com\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_res = Response::try_from(&input[..]).unwrap();

        let mut headers = Headers::new();
        headers.add_content_length(0);
        headers.add_connection("keep-alive");
        headers.insert(SERVER, "example.com".into());
        headers.insert("Pineapple".into(), "pizza".into());

        let expected_res = Response {
            headers,
            ..Response::default()
        };

        assert_eq!(test_res, expected_res);
    }
}

#[cfg(test)]
mod trim {
    use crate::utils::Trim;

    #[test]
    fn whitespace_bytes() {
        assert_eq!(b"  test".trim(), b"test");
        assert_eq!(b"test    ".trim(), b"test");
        assert_eq!(b"         test       ".trim(), b"test");
        assert_eq!(b"                   ".trim(), b"");
        assert_eq!(b"\t  \nx\t  x\r\x0c".trim(), b"x\t  x");
        assert_eq!(b"test".trim(), b"test");
        assert_eq!(b"".trim(), b"");
    }
}

mod styles {
    macro_rules! test_format_str_parsing {
        ($( $format_str:literal: $req_style:expr, $res_style:expr; )+) => {
            #[test]
            fn from_format_str() {
                use $crate::{Style, StyleKind::*, StyleParts::*};

                $(
                    let expected = Style { req: $req_style, res: $res_style };

                    let mut test = Style::default();
                    test.from_format_str($format_str);

                    assert_eq!(test, expected);
                )+
            }
        };
    }

    test_format_str_parsing! {
        "": Color(None), Color(None);
        "R": Color(Line), Color(None);
        "H": Color(Hdrs), Color(None);
        "B": Color(Body), Color(None);
        "s": Color(None), Color(Line);
        "h": Color(None), Color(Hdrs);
        "b": Color(None), Color(Body);
        "*": Color(All), Color(All);
        "Rs*": Color(All), Color(All);
        "RHBshb": Color(All), Color(All);
        "Rs": Color(Line), Color(Line);
        "Hh": Color(Hdrs), Color(Hdrs);
        "Bb": Color(Body), Color(Body);
        "RHs": Color(LineHdrs), Color(Line);
        "RBs": Color(LineBody), Color(Line);
        "HBs": Color(HdrsBody), Color(Line);
        "Rsh": Color(Line), Color(LineHdrs);
        "Rsb": Color(Line), Color(LineBody);
        "Rhb": Color(Line), Color(HdrsBody);
        "RHB": Color(All), Color(None);
        "shb": Color(None), Color(All);
        "xyz3s": Color(None), Color(Line);
    }
}

mod trait_impls {
    use super::*;
    use std::error::Error;

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
        Body, Client, Connection, Header, Headers, Method, NetError,
        NetParseError, Request, Response, Route, Router, Server,
        ServerHandle<()>, Status, Target, Version
    ];
    trait_impl_test! [sync_types implement Sync:
        Body, Client, Connection, Header, Headers, Method, NetError,
        NetParseError, Request, Response, Route, Router, Server,
        ServerHandle<()>, Status, Target, Version
    ];
    trait_impl_test! [error_types implement Error:
        NetError, NetParseError
    ];
}
