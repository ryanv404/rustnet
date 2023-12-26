#![allow(unused)]

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use rustnet::header_name::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC,
    ACCESS_CONTROL_ALLOW_ORIGIN as ACAO, CONNECTION, CONTENT_LENGTH,
    CONTENT_TYPE, HOST, LOCATION, SERVER,
};
use rustnet::{
    Body, Connection, Header, Headers, Method, Request, RequestLine,
    Response, Status, StatusCode, StatusLine, Version,
};

// Start a test server in the background.
macro_rules! start_test_server {
    () => {
        #[test]
        fn start_test_server() {
            let args = [
                "run", "--bin", "server", "--",
                "--test", "--",
                "127.0.0.1:7878"
            ];

            let _server = Command::new("cargo")
                .args(&args[..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();

            // Test fails if server is not live.
            if !server_is_live(false) {
                assert!(false);
            }
        }
    };
}

// Shut down the test server using a shutdown route.
macro_rules! shutdown_test_server {
    () => {
        #[test]
        fn shutdown_test_server() {
            let args = [
                "run", "--bin", "client", "--",
                "--shutdown", "--",
                "127.0.0.1:7878"
            ];

            let _shutdown = Command::new("cargo")
                .args(&args[..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();

            // Test fails if server is still live.
            if server_is_live(true) {
                assert!(false);
            }
        }
    };
}

macro_rules! run_test {
    ($( $kind:ident: $method:ident $route:ident )+) => {
        $(
            #[test]
            fn $route() {
                let method = stringify!($method);
                let test_kind = stringify!($kind);

                let route = concat!("/", stringify!($route));
                let route = if route == "/many_methods" {
                    String::from(route)
                } else {
                    route.replace("_", "/")
                };

                let addr = match test_kind {
                    "CLIENT" => "httpbin.org:80",
                    "SERVER" => "127.0.0.1:7878",
                    _ => unreachable!(),
                };

                let args = [
                    "run", "--bin", "client", "--",
                    "--method", method, "--path", route.as_str(), "--output", "sh",
                    "--plain", "--no-dates", "--", addr
                ];

                let output = Command::new("cargo")
                    .args(&args[..])
                    .output()
                    .unwrap();

                let test_res = match Response::try_from(&output.stdout[..]) {
                    Ok(mut res) => {
                        res.body = Body::Empty;
                        res
                    },
                    Err(e) => panic!("Response parsing failed!\n{e}"),
                };

                let expected_res = match test_kind {
                    "SERVER" => get_expected_for_server(method, route.as_str()),
                    "CLIENT" => get_expected_for_client(method, route.as_str()),
                    _ => unreachable!(),
                };

                assert_eq!(test_res, expected_res);
            }
        )+
    };
}

pub fn server_is_live(is_shutting_down: bool) -> bool {
    let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
    let socket = SocketAddr::new(ip, 7878);
    let timeout = Duration::from_millis(200);

    for _ in 0..5 {
        if TcpStream::connect_timeout(&socket, timeout).is_ok() {
            if !is_shutting_down {
                // Returns success state for a server starting up.
                return true;
            }
        } else if is_shutting_down {
            // Returns success state for a server shutting down.
            return false;
        }

        thread::sleep(timeout);
    }

    // Returns the fail state:
    // - True (server is live) if server is shutting down.
    // - False (server is not live) if server is starting up.
    is_shutting_down
}

pub fn favicon_route() -> Response {
    let mut res = Response::default();
    res.headers.content_length(1406);
    res.headers.server("rustnet/0.1");
    res.headers.content_type("image/x-icon");
    res.headers.cache_control("max-age=604800");
    res
}

pub fn many_methods(content_len: usize, code: u16) -> Response {
    let mut res = Response::default();
    res.status_line.status = Status(StatusCode(code));
    res.headers.server("rustnet/0.1");
    res.headers.cache_control("no-cache");
    res.headers.content_length(content_len);
    res.headers.content_type("text/plain; charset=utf-8");
    res
}

pub fn unknown_route() -> Response {
    let mut res = Response::default();
    res.status_line.status = Status(StatusCode(404));
    res.headers.content_length(482);
    res.headers.server("rustnet/0.1");
    res.headers.cache_control("no-cache");
    res.headers.content_type("text/html; charset=utf-8");
    res
}

pub fn known_route(code: u16, len: usize) -> Response {
    let mut res = Response::default();
    res.status_line.status = Status(StatusCode(code));
    res.headers.content_length(len);
    res.headers.server("rustnet/0.1");
    res.headers.cache_control("no-cache");
    res.headers.content_type("text/html; charset=utf-8");
    res
}

pub fn get_expected_for_client(method: &str, route: &str) -> Response {
//    let mut req_headers = Headers::new();
//    req_headers.accept("*/*");
//    req_headers.user_agent("rustnet/0.1");
//    req_headers.insert(HOST, "httpbin.org:80".into());
//
//    let method = Method::from_str(method_str).unwrap();
//
//    Request {
//        request_line: RequestLine {
//            method,
//            path: path_str.to_string(),
//            version: Version::OneDotOne
//        },
//        headers: req_headers,
//        body: Body::Empty
//    }

    let mut res = Response::default();
    res.headers.insert(ACAO, "*".into());
    res.headers.insert(ACAC, "true".into());
    res.headers.insert(CONTENT_LENGTH, "0".into());
    res.headers.insert(CONNECTION, "keep-alive".into());
    res.headers.insert(SERVER, "gunicorn/19.9.0".into());
    res.headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".into());

    match route {
        "/status/101" => {
            res.headers.remove(&CONTENT_LENGTH);
            res.status_line = StatusLine::from(StatusCode(101));
            res.headers.insert(CONNECTION, "upgrade".into());
        },
        "/status/301" => {
            res.headers.remove(&CONTENT_TYPE);
            res.headers.insert(LOCATION, "/redirect/1".into());
            res.status_line = StatusLine::from(StatusCode(301));
        },
        "/status/404" => {
            res.status_line = StatusLine::from(StatusCode(404));
        },
        "/status/502" => {
            res.status_line = StatusLine::from(StatusCode(502));
        },
        "/xml" => {
            res.headers.content_length(522);
            res.headers.content_type("application/xml");
        },
        "/json" => {
            res.headers.content_length(429);
            res.headers.content_type("application/json");
        },
        "/deny" => {
            res.headers.content_length(239);
            res.headers.content_type("text/plain");
        },
        "/html" => {
            res.headers.content_length(3741);
            res.headers.content_type("text/html; charset=utf-8");
        },
        "/image/jpeg" => {
            res.headers.content_length(35588);
            res.headers.content_type("image/jpeg");
        },
        _ => {},
    }

    res
}

pub fn get_expected_for_server(method: &str, route: &str) -> Response {
    match (method, route) {
        (_, "/foo") => unknown_route(),
        (_, "/favicon.ico") => favicon_route(),
        ("HEAD", "/many_methods") => many_methods(23, 200),
        ("POST", "/many_methods") => many_methods(23, 201),
        ("DELETE", "/many_methods") => many_methods(25, 200),
        ("GET" | "PUT", "/many_methods") => many_methods(22, 200),
        ("PATCH" | "TRACE", "/many_methods") => many_methods(24, 200),
        ("OPTIONS" | "CONNECT", "/many_methods") => many_methods(26, 200),
        (_, "/about") => known_route(200, 455),
        (_, "/post") =>  known_route(201, 575),
        (_, "/get"
            | "/put" | "/options" | "/connect" | "/delete" | "/head"
            | "/patch" | "/trace") => known_route(200, 575),
        (_, _) => panic!("Unexpected method or route: {method:?}, {route:?}"),
    }
}
