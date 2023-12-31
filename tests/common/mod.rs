#![allow(unused_macros)]

use rustnet::{Request, Response};

// Start a test server in the background.
macro_rules! start_test_server {
    () => {
        #[test]
        fn start_test_server() {
            use std::process::{Command, Stdio};
            use rustnet::TEST_SERVER_ADDR;
            use rustnet::util;

            let args = [
                "run", "--bin", "server", "--",
                "--test", "--",
                TEST_SERVER_ADDR
            ];

            let _ = Command::new("cargo")
                .args(&args[..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();

            if !util::check_server(TEST_SERVER_ADDR) {
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
            use std::process::{Command, Stdio};
            use std::thread;
            use std::time::Duration;
            use rustnet::TEST_SERVER_ADDR;
            use rustnet::util;

            let args = [
                "run", "--bin", "client", "--",
                "--shutdown", "--",
                TEST_SERVER_ADDR
            ];

            let _ = Command::new("cargo")
                .args(&args[..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();

            thread::sleep(Duration::from_millis(400));

            if util::check_server(TEST_SERVER_ADDR) {
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
                use std::process::Command;
                use rustnet::{Body, Response, TEST_SERVER_ADDR};
                use common::{get_client_expected, get_server_expected};

                let method = stringify!($method);

                let route = match concat!("/", stringify!($route)) {
                    "/many_methods" => String::from("/many_methods"),
                    "/known" => format!("/{}", method.to_ascii_lowercase()),
                    "/unknown" => String::from("/unknown"),
                    route => route.replace("_", "/"),
                };

                let (addr, expected_res) = match stringify!($kind) {
                    "CLIENT" => {
                        let (_req, res) = get_client_expected(method, &route);
                        ("httpbin.org:80", res)
                    },
                    "SERVER" => {
                        let res = get_server_expected(method, &route);
                        (TEST_SERVER_ADDR, res)
                    },
                    _ => unreachable!(),
                };

                let args = [
                    "run", "--bin", "client", "--",
                    "--method", method,
                    "--path", route.as_str(),
                    "--output", "sh",
                    "--plain", "--no-dates", "--",
                    addr
                ];

                let output = Command::new("cargo")
                    .args(&args[..])
                    .output()
                    .unwrap();

                match Response::try_from(&output.stdout[..]) {
                    Ok(mut test_res) => {
                        test_res.body = Body::Empty;
                        assert_eq!(test_res, expected_res);
                    },
                    Err(e) => panic!("Response parsing failed!\n{e}"),
                }
            }
        )+
    };
}

pub fn get_known_route_res(route: &str) -> Response {
    use rustnet::{Status, DEFAULT_NAME};

    let mut res = Response::new();
    res.headers.server(DEFAULT_NAME);
    res.headers.cache_control("no-cache");
    res.headers.content_type("text/html; charset=utf-8");

    match route {
        "/about" => {
            res.headers.content_length(455);
            res.status_line.status = Status(200u16);
        },
        "/post" => {
            res.headers.content_length(575);
            res.status_line.status = Status(201u16);
        },
        "/get"
            | "/head"
            | "/put"
            | "/patch"
            | "/delete"
            | "/trace"
            | "/options"
            | "/connect" =>
        {
            res.headers.content_length(575);
            res.status_line.status = Status(200u16);
        },
        _ => unreachable!(),
    }

    res
}

pub fn get_unknown_route_res() -> Response {
    use rustnet::{Status, DEFAULT_NAME};

    let mut res = Response::new();
    res.headers.server(DEFAULT_NAME);
    res.headers.content_length(482);
    res.headers.cache_control("no-cache");
    res.status_line.status = Status(404u16);
    res.headers.content_type("text/html; charset=utf-8");
    res
}

pub fn get_favicon_route_res() -> Response {
    use rustnet::{DEFAULT_NAME};

    let mut res = Response::new();
    res.headers.server(DEFAULT_NAME);
    res.headers.content_length(1406);
    res.headers.content_type("image/x-icon");
    res.headers.cache_control("max-age=604800");
    res
}

pub fn get_many_methods_route_res(method: &str) -> Response {
    use rustnet::{Status, DEFAULT_NAME};

    let mut res = Response::new();
    res.headers.server(DEFAULT_NAME);
    res.headers.cache_control("no-cache");
    res.headers.content_type("text/plain; charset=utf-8");

    match method {
        "HEAD" => {
            res.headers.content_length(23);
            res.status_line.status = Status(200u16);
        },
        "POST" => {
            res.headers.content_length(23);
            res.status_line.status = Status(201u16);
        },
        "DELETE" => {
            res.headers.content_length(25);
            res.status_line.status = Status(200u16);
        },
        "GET" | "PUT" => {
            res.headers.content_length(22);
            res.status_line.status = Status(200u16);
        },
        "PATCH" | "TRACE" => {
            res.headers.content_length(24);
            res.status_line.status = Status(200u16);
        },
        "OPTIONS" | "CONNECT" => {
            res.headers.content_length(26);
            res.status_line.status = Status(200u16);
        },
        _ => unreachable!(),
    }

    res
}

pub fn get_client_expected(method: &str, route: &str) -> (Request, Response) {
    use std::str::FromStr;
    use rustnet::{Method, Status, DEFAULT_NAME};
    use rustnet::header::names::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC,
        ACCESS_CONTROL_ALLOW_ORIGIN as ACAO, CONTENT_LENGTH, CONTENT_TYPE,
        HOST, LOCATION,
    };

    let mut req = Request::new();
    req.headers.accept("*/*");
    req.request_line.path = route.into();
    req.headers.user_agent(DEFAULT_NAME);
    req.headers.insert(HOST, "httpbin.org:80".into());
    req.request_line.method = Method::from_str(method).unwrap();

    let mut res = Response::new();
    res.headers.content_length(0);
    res.headers.insert(ACAO, "*".into());
    res.headers.connection("keep-alive");
    res.headers.server("gunicorn/19.9.0");
    res.headers.insert(ACAC, "true".into());
    res.headers.content_type("text/html; charset=utf-8");

    match route {
        "/status/101" => {
            res.headers.connection("upgrade");
            res.headers.remove(&CONTENT_LENGTH);
            res.status_line.status = Status(101u16);
        },
        "/status/201" => {
            res.status_line.status = Status(201u16);
        },
        "/status/301" => {
            res.headers.remove(&CONTENT_TYPE);
            res.status_line.status = Status(301u16);
            res.headers.insert(LOCATION, "/redirect/1".into());
        },
        "/status/404" => {
            res.status_line.status = Status(404u16);
        },
        "/status/502" => {
            res.status_line.status = Status(502u16);
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

    (req, res)
}

pub fn get_server_expected(method: &str, route: &str) -> Response {
    match route {
        "/unknown" => get_unknown_route_res(),
        "/favicon.ico" => get_favicon_route_res(),
        "/many_methods" => get_many_methods_route_res(method),
        _ if route.starts_with('/') => get_known_route_res(route),
        _ => unreachable!(),
    }
}
