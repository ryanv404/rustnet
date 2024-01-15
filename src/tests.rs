use std::num::NonZeroU16;
use std::str::FromStr;

use crate::{
    Body, Client, Connection, Headers, Method, NetError, NetParseError,
    Request, Response, Route, Router, Server, ServerHandle, Status, Target,
    UriPath, Version,
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

macro_rules! test_parsing_from_bytes {
    (
        $target:ident $label:ident:
            $( $input:literal => $expected:expr; )+
            $( BAD_INPUT: $bad_input:literal; )*
    ) => {
        #[test]
        fn $label() {
            $( assert_eq!($target::try_from(&$input[..]), Ok($expected)); )+
            $( assert!($target::try_from(&$bad_input[..]).is_err()); )*
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
mod method {
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

    test_parsing_from_bytes! {
        Method from_bytes:
        b"ANY" => Method::Any;
        b"GET" => Method::Get;
        b"HEAD" => Method::Head;
        b"POST" => Method::Post;
        b"PUT" => Method::Put;
        b"PATCH" => Method::Patch;
        b"DELETE" => Method::Delete;
        b"TRACE" => Method::Trace;
        b"OPTIONS" => Method::Options;
        b"CONNECT" => Method::Connect;
        b"SHUTDOWN" => Method::Shutdown;
        BAD_INPUT: b"Foo";
        BAD_INPUT: b"get";
    }
}

#[cfg(test)]
mod status {
    use super::*;

    test_parsing_from_str! {
        Status from_str:
        "103" => Status(NonZeroU16::new(103u16).unwrap());
        "211" => Status(NonZeroU16::new(211u16).unwrap());
        "302" => Status(NonZeroU16::new(302u16).unwrap());
        "404" => Status(NonZeroU16::new(404u16).unwrap());
        "503" => Status(NonZeroU16::new(503u16).unwrap());
        BAD_INPUT: "01";
        BAD_INPUT: "a202";
    }

    test_parsing_from_bytes! {
        Status from_bytes:
        b"101" => Status(NonZeroU16::new(101u16).unwrap());
        b"201" => Status(NonZeroU16::new(201u16).unwrap());
        b"300" => Status(NonZeroU16::new(300u16).unwrap());
        b"400" => Status(NonZeroU16::new(400u16).unwrap());
        b"501" => Status(NonZeroU16::new(501u16).unwrap());
        BAD_INPUT: b"1234";
        BAD_INPUT: b"abc";
    }

    test_parsing_from_int! {
        Status from_int:
        201u16 => Status(NonZeroU16::new(201u16).unwrap());
        202u16 => Status(NonZeroU16::new(202u16).unwrap());
        203u16 => Status(NonZeroU16::new(203u16).unwrap());
        BAD_INPUT: 1001u16;
        BAD_INPUT: 99u16;
        BAD_INPUT: 0u16;
    }
}

#[cfg(test)]
mod version {
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

    test_parsing_from_bytes! {
        Version from_bytes:
        b"HTTP/0.9" => Version::ZeroDotNine;
        b"HTTP/1.0" => Version::OneDotZero;
        b"HTTP/1.1" => Version::OneDotOne;
        b"HTTP/2.0" => Version::TwoDotZero;
        b"HTTP/2" => Version::TwoDotZero;
        b"HTTP/3.0" => Version::ThreeDotZero;
        b"HTTP/3" => Version::ThreeDotZero;
        BAD_INPUT: b"HTTP/1.2";
        BAD_INPUT: b"HTTP/1.10";
    }
}

#[cfg(test)]
mod standard_headers {
    use crate::HeaderName;
    use crate::headers::names::{HeaderNameInner, STD_HEADER_NAMES};

    #[test]
    fn from_str() {
        for &(std, titlecase) in STD_HEADER_NAMES {
            let uppercase = titlecase.to_ascii_uppercase();
            let uppercase_test = HeaderName::from(uppercase.as_str());
            let uppercase_expected = HeaderName {
                inner: HeaderNameInner::Standard(std)
            };

            let titlecase_test = HeaderName::from(titlecase);
            let titlecase_expected = HeaderName {
                inner: HeaderNameInner::Standard(std)
            };

            assert_eq!(uppercase_test, uppercase_expected);
            assert_eq!(titlecase_test, titlecase_expected);
        }
    }
}

#[cfg(test)]
mod multiple_headers {
    use super::*;
    use crate::headers::names::{
        ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, USER_AGENT,
    };

    #[test]
    fn from_str() {
        let input = "\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_headers = Headers::from_str(input).unwrap();

        let mut expected_headers = Headers::new();
        expected_headers.insert(ACCEPT, "*/*".into());
        expected_headers.insert(HOST, "example.com".into());
        expected_headers.insert(USER_AGENT, "xh/0.19.3".into());
        expected_headers.insert(CONNECTION, "keep-alive".into());
        expected_headers.insert("Pineapple".into(), "pizza".into());
        expected_headers.insert(ACCEPT_ENCODING, "gzip, deflate, br".into());

        assert_eq!(test_headers, expected_headers);
    }

    #[test]
    fn from_bytes() {
        let input = b"\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_headers = Headers::try_from(&input[..]).unwrap();

        let mut expected_headers = Headers::new();
        expected_headers.insert(ACCEPT, "*/*".into());
        expected_headers.insert(HOST, "example.com".into());
        expected_headers.insert(USER_AGENT, "xh/0.19.3".into());
        expected_headers.insert(CONNECTION, "keep-alive".into());
        expected_headers.insert("Pineapple".into(), "pizza".into());
        expected_headers.insert(ACCEPT_ENCODING, "gzip, deflate, br".into());

        assert_eq!(test_headers, expected_headers);
    }
}

#[cfg(test)]
mod http_uri {
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
mod request {
    use super::*;
    use crate::headers::names::{
        ACCEPT, ACCEPT_ENCODING, CONNECTION, CONTENT_LENGTH, HOST, USER_AGENT,
    };

    #[test]
    fn from_str() {
        let input = "\
            GET /test HTTP/1.1\r\n\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Content-Length: 0\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_req = Request::from_str(input).unwrap();

        let mut expected_req = Request::default();
        expected_req.path = UriPath("/test".into());
        expected_req.headers.insert(ACCEPT, "*/*".into());
        expected_req.headers.insert(CONTENT_LENGTH, 0.into());
        expected_req.headers.insert(HOST, "example.com".into());
        expected_req.headers.insert(USER_AGENT, "xh/0.19.3".into());
        expected_req.headers.insert(CONNECTION, "keep-alive".into());
        expected_req.headers.insert("Pineapple".into(), "pizza".into());
        expected_req.headers.insert(ACCEPT_ENCODING, "gzip, deflate, br".into());

        assert_eq!(test_req, expected_req);
    }

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

        let mut expected_req = Request::default();
        expected_req.path = UriPath("/test".into());
        expected_req.headers.insert(ACCEPT, "*/*".into());
        expected_req.headers.insert(CONTENT_LENGTH, 0.into());
        expected_req.headers.insert(HOST, "example.com".into());
        expected_req.headers.insert(USER_AGENT, "xh/0.19.3".into());
        expected_req.headers.insert(CONNECTION, "keep-alive".into());
        expected_req.headers.insert("Pineapple".into(), "pizza".into());
        expected_req.headers.insert(ACCEPT_ENCODING, "gzip, deflate, br".into());

        assert_eq!(test_req, expected_req);
    }
}

#[cfg(test)]
mod response {
    use super::*;
    use crate::headers::names::{CONNECTION, CONTENT_LENGTH, SERVER};

    #[test]
    fn from_str() {
        let input = "\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 0\r\n\
            Connection: keep-alive\r\n\
            Server: example.com\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_res = Response::from_str(input).unwrap();

        let mut expected_res = Response::default();
        expected_res.headers.insert(CONTENT_LENGTH, 0.into());
        expected_res.headers.insert(SERVER, "example.com".into());
        expected_res.headers.insert(CONNECTION, "keep-alive".into());
        expected_res.headers.insert("Pineapple".into(), "pizza".into());

        assert_eq!(test_res, expected_res);
    }

    #[test]
    fn from_bytes() {
        let input = b"\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 0\r\n\
            Connection: keep-alive\r\n\
            Server: example.com\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_res = Response::try_from(&input[..]).unwrap();

        let mut expected_res = Response::default();
        expected_res.headers.insert(CONTENT_LENGTH, 0.into());
        expected_res.headers.insert(SERVER, "example.com".into());
        expected_res.headers.insert(CONNECTION, "keep-alive".into());
        expected_res.headers.insert("Pineapple".into(), "pizza".into());

        assert_eq!(test_res, expected_res);
    }
}

#[cfg(test)]
mod utils {
    use crate::utils::{to_titlecase, trim};

    #[test]
    fn make_titlecase() {
        assert_eq!(to_titlecase("test"),
            "Test".to_string());
        assert_eq!(to_titlecase(" two-parts "),
            "Two-Parts".to_string());
        assert_eq!(to_titlecase("maNy-PArts-in-tHIS"),
            "Many-Parts-In-This".to_string());
        assert_eq!(to_titlecase("how-3about-with-0nums"),
            "How-3about-With-0nums".to_string());
        assert_eq!(to_titlecase(""), "".to_string());
    }

    #[test]
    fn trim_white_space() {
        assert_eq!(trim(b""), b"");
        assert_eq!(trim(b"test"), b"test");
        assert_eq!(trim(b"  test"), b"test");
        assert_eq!(trim(b"test    "), b"test");
        assert_eq!(trim(b"                   "), b"");
        assert_eq!(trim(b"         test       "), b"test");
        assert_eq!(trim(b"\t  \nx\t  x\r\x0c"), b"x\t  x");
    }
}

mod style {
    macro_rules! test_format_str_parsing {
        ($( $format_str:literal: $req_style:expr, $res_style:expr; )+) => {
            #[test]
            fn from_format_str() {
                use $crate::{Style, StyleKind::*, StyleParts::*};

                $(
                    let expected_style = Style {
                        req: $req_style,
                        res: $res_style
                    };

                    let mut test_style = Style::default();
                    test_style.from_format_str($format_str);

                    assert_eq!(test_style, expected_style);
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
        Body, Client, Connection, Headers, Method, NetError,
        NetParseError, Request, Response, Route, Router, Server,
        ServerHandle<()>, Status, Target, Version
    ];
    trait_impl_test! [sync_types implement Sync:
        Body, Client, Connection, Headers, Method, NetError,
        NetParseError, Request, Response, Route, Router, Server,
        ServerHandle<()>, Status, Target, Version
    ];
    trait_impl_test! [error_types implement Error:
        NetError, NetParseError
    ];
}
