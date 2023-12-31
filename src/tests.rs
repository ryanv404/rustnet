use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::path::PathBuf;
use std::str::{self, FromStr};

use crate::{
    Body, Client, ClientCli, Connection, Header, HeaderName, HeaderNameInner,
    HeaderValue, Headers, Method, NetError, NetParseError, NetResult,
    Request, RequestBuilder, RequestLine, Response, ResponseBuilder, Route,
    RouteBuilder, Router, Server, ServerBuilder, ServerCli, ServerHandle,
    Status, StatusLine, Style, StyleKind, StyleParts, Target,
    ThreadPool, Version, Worker, DEFAULT_NAME,
};
use crate::header::names::{
    ACCEPT, ACCEPT_ENCODING, CACHE_CONTROL, CONNECTION, CONTENT_LENGTH,
    CONTENT_TYPE, HOST, SERVER, STANDARD_HEADERS, USER_AGENT,
};
use crate::util::{self, trim, trim_start, trim_end};

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
mod method {
    use super::*;

    test_parsing_from_str! {
        Method from_str:
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
mod status {
    use super::*;

    test_parsing_from_str! {
        Status from_str:
        "101 Switching Protocols" => Status(101u16);
        "201 Created" => Status(201u16);
        "300 Multiple Choices" => Status(300u16);
        "400 Bad Request" => Status(400u16);
        "501 Not Implemented" => Status(501u16);
        BAD_INPUT: "1234 Bad Status";
        BAD_INPUT: "abc";
    }

    test_parsing_from_int! {
        Status from_int:
        201_u16 => Status(201u16);
        202_u32 => Status(202u16);
        203_i32 => Status(203u16);
        BAD_INPUT: 1001_u16;
        BAD_INPUT: -123_i32;
        BAD_INPUT: 0_u16;
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
}

#[cfg(test)]
mod request_line {
    use super::*;

    test_parsing_from_str! {
        RequestLine from_str:
        "GET /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Get, "/test");
        "HEAD /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Head, "/test");
        "POST /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Post, "/test");
        "PUT /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Put, "/test");
        "PATCH /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Patch, "/test");
        "DELETE /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Delete, "/test");
        "TRACE /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Trace, "/test");
        "OPTIONS /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Options, "/test");
        "CONNECT /test HTTP/1.1\r\n" =>
            RequestLine::new(Method::Connect, "/test");
        "SHUTDOWN / HTTP/1.1\r\n" =>
            RequestLine::new(Method::Shutdown, "/");
        BAD_INPUT: "FOO bar baz";
        BAD_INPUT: "GET /test";
        BAD_INPUT: "GET";
    }
}

#[cfg(test)]
mod status_line {
    use super::*;

    test_parsing_from_str! {
        StatusLine from_str:
        "HTTP/1.1 100 Continue\r\n" =>
            StatusLine::try_from(100u16).unwrap();
        "HTTP/1.1 200 OK\r\n" =>
            StatusLine::try_from(200u16).unwrap();
        "HTTP/1.1 301 Moved Permanently\r\n" =>
            StatusLine::try_from(301u16).unwrap();
        "HTTP/1.1 403 Forbidden\r\n" =>
            StatusLine::try_from(403u16).unwrap();
        "HTTP/1.1 505 HTTP Version Not Supported\r\n" =>
            StatusLine::try_from(505u16).unwrap();
        BAD_INPUT: "HTTP/1.1";
        BAD_INPUT: "200 OK";
        BAD_INPUT: "FOO bar baz";
    }
}

#[cfg(test)]
mod header {
    use super::*;

    test_parsing_from_str! {
        Header from_str:
        "Accept: */*\r\n" =>
            Header { name: ACCEPT, value: "*/*".into() };
        "Host: rustnet/0.1.0\r\n" =>
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
mod standard_headers {
    use super::*;

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
mod uri {
    use super::*;

    #[test]
    fn from_str() {
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
mod headers {
    use super::*;

    #[test]
    fn from_str() {
        let test_hdrs = Headers::from_str("\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n"
        )
        .unwrap();

        let expected_hdrs = Headers(BTreeMap::from([
            (ACCEPT, "*/*".into()),
            (HOST, "example.com".into()),
            (USER_AGENT, "xh/0.19.3".into()),
            (CONNECTION, "keep-alive".into()),
            ("Pineapple".into(), "pizza".into()),
            (ACCEPT_ENCODING, "gzip, deflate, br".into())
        ]));

        assert_eq!(test_hdrs, expected_hdrs);
    }
}

#[cfg(test)]
mod request {
    use super::*;
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

        let mut expected_req = Request::new();
        expected_req.request_line.path = "/test".into();
        expected_req.headers.insert(ACCEPT, "*/*".into());
        expected_req.headers.insert(HOST, "example.com".into());
        expected_req.headers.insert(CONTENT_LENGTH, "0".into());
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

    #[test]
    fn from_bytes() {
        let input = b"\
            HTTP/1.1 200 OK\r\n\
            Content-Length: 0\r\n\
            Connection: keep-alive\r\n\
            Server: example.com\r\n\
            Pineapple: pizza\r\n\r\n";

        let test_res = Response::try_from(&input[..]).unwrap();

        let mut expected_res = Response::new();
        expected_res.headers.insert(SERVER, "example.com".into());
        expected_res.headers.insert(CONTENT_LENGTH, "0".into());
        expected_res.headers.insert(CONNECTION, "keep-alive".into());
        expected_res.headers.insert("Pineapple".into(), "pizza".into());

        assert_eq!(test_res, expected_res);
    }
}

#[cfg(test)]
mod utils {
    use super::*;

    #[test]
    fn test_trim() {
        assert_eq!(trim(b"  test"), b"test");
        assert_eq!(trim(b"test    "), b"test");
        assert_eq!(trim(b"         test       "), b"test");
        assert_eq!(trim(b"\t  \nx\t  x\r\x0c"), b"x\t  x");
        assert_eq!(trim(b"                   "), b"");
        assert_eq!(trim(b"x"), b"x");
        assert_eq!(trim(b""), b"");
    }

    #[test]
    fn test_trim_start() {
        assert_eq!(trim_start(b"  test"), b"test");
        assert_eq!(trim_start(b"test    "), b"test    ");
        assert_eq!(trim_start(b"         test       "), b"test       ");
        assert_eq!(trim_start(b"                   "), b"");
    }

    #[test]
    fn test_trim_end() {
        assert_eq!(trim_end(b"  test"), b"  test");
        assert_eq!(trim_end(b"test    "), b"test");
        assert_eq!(trim_end(b"         test       "), b"         test");
        assert_eq!(trim_end(b"                   "), b"");
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
        NetParseError, Request, RequestBuilder, RequestLine, Response,
        ResponseBuilder, Route, RouteBuilder, Router, Server, ServerBuilder,
        ServerHandle<()>, Status, StatusLine, Style, Target,
        ThreadPool, Version, Worker];
    trait_impl_test! [sync_types implement Sync:
        Body, Client, Connection, Header, HeaderNameInner, HeaderName,
        HeaderValue, Headers, Method, NetError, NetResult<()>,
        NetParseError, Request, RequestBuilder, RequestLine, Response,
        ResponseBuilder, Route, RouteBuilder, Router, Server,
        ServerBuilder, ServerHandle<()>, Status, StatusLine,
        Style, Target, ThreadPool, Version, Worker];
    trait_impl_test! [error_types implement Error:
        NetError, NetParseError];
}

mod style {
    use super::*;

    macro_rules! test_format_str_parsing {
        ($( $format_str:literal: $req_style:expr, $res_style:expr; )+) => {
            #[test]
            fn from_format_str() {
                use self::{StyleKind::*, StyleParts::*};

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
        "R": Color(Line), Color(None);
        "H": Color(Hdrs), Color(None);
        "B": Color(Body), Color(None);
        "s": Color(None), Color(Line);
        "h": Color(None), Color(Hdrs);
        "b": Color(None), Color(Body);
        "Rs": Color(Line), Color(Line);
        "RHB": Color(All), Color(None);
        "shb": Color(None), Color(All);
        "RHsh": Color(LineHdrs), Color(LineHdrs);
        "HBsb": Color(HdrsBody), Color(LineBody);
        "RBsb": Color(LineBody), Color(LineBody);
        "RHBshb": Color(All), Color(All);
        "*": Color(All), Color(All);
        "": Color(None), Color(None);
        "xyz3s": Color(None), Color(Line);
    }
}

mod client_cli {
    use super::*;

    #[test]
    fn parse_args() {
        let mut args = VecDeque::from([
            "./client",
            "--plain",
            "--no-dates",
            "--output", "/Bs2Rhb1",
            "--method", "posT",
            "-H", "acCEpT:*/*",
            "-H", "conteNt-leNgth:13",
            "-H", "caCHe-controL:no-CACHE",
            "--debug",
            "-H", "cOntent-tYpe:text/html; charset=utf-8",
            "-H", "pineaPPle:yUm123",
            "--body", "This is a test meSSage :) in the request bOdy.",
            "httpbin.org/json"
        ]);

        let expected_cli = ClientCli {
            no_dates: true,
            do_debug: true,
            do_plain: true,
            style: Style {
                req: StyleKind::Plain(StyleParts::LineBody),
                res: StyleKind::Plain(StyleParts::All)
            },
            method: Method::Post,
            path: "/json".into(),
            addr: Some("httpbin.org:80".to_string()),
            headers: Headers(BTreeMap::from([
                (ACCEPT, "*/*".into()),
                (CONTENT_LENGTH, "13".into()),
                (CACHE_CONTROL, "no-cache".into()),
                (util::to_titlecase(b"Pineapple").into(),
                    "yum123".into()),
                (CONTENT_TYPE, "text/html; charset=utf-8".into()),
            ])),
            body: Body::Text(
                Vec::from("This is a test meSSage :) in the request bOdy.")
            ),
            ..ClientCli::default()
        };

        let test_cli = ClientCli::parse_args(&mut args).unwrap();

        assert_eq!(test_cli, expected_cli);

        let mut test_client = Client::try_from(test_cli).unwrap();
        let mut expected_client = Client::try_from(expected_cli).unwrap();

        if let Some(req) = test_client.req.as_mut() {
            req.headers.header("Host", "httpbin.org");
        }

        if let Some(req) = expected_client.req.as_mut() {
            req.headers.header("Host", "httpbin.org");
        }

        assert_eq!(test_client, expected_client);
    }
}

mod server_cli {
    use super::*;

    #[test]
    fn parse_args() {
        let mut args = VecDeque::from([
            "./server", "--test", "-d",
            "--log-file", "./log_file.txt",
            "-I", "./favicon.ico",
            "-0", "./static/error_404.html",
            "-T", "pUt:/put:test message1.",
            "-T", "pAtch:/patCh:test message2.",
            "-T", "DeleTe:/dEletE:test message3.",
            "-F", "GeT:/geT:./static/get.html",
            "-F", "HEaD:/hEad:./static/head.html",
            "-F", "pOst:/poSt:./static/post.html",
            "127.0.0.1:7879"
        ]);

        let test_cli = ServerCli::parse_args(&mut args);

        let expected_cli = ServerCli {
            do_log: true,
            do_debug: true,
            is_test: true,
            addr: Some("127.0.0.1:7879".to_string()),
            log_file: Some(PathBuf::from("./log_file.txt")),
            router: Router(BTreeMap::from([
                (Route::Shutdown, Target::Shutdown),
                (Route::NotFound,
                    Target::File("./static/error_404.html".into())),
                (Route::Get("/get".into()),
                    Target::File("./static/get.html".into())),
                (Route::Put("/put".into()),
                    Target::Text("test message1.".to_string())),
                (Route::Post("/post".into()),
                    Target::File("./static/post.html".into())),
                (Route::Head("/head".into()),
                    Target::File("./static/head.html".into())),
                (Route::Get("/favicon.ico".into()),
                    Target::File("./favicon.ico".into())),
                (Route::Patch("/patch".into()),
                    Target::Text("test message2.".to_string())),
                (Route::Delete("/delete".into()),
                    Target::Text("test message3.".to_string())),
            ]))
        };

        assert_eq!(test_cli, expected_cli);
    }
}
