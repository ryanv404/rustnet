#![allow(unused_macros)]

use rustnet::Response;

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
                    "/unknown" => String::from("/unknown"),
                    "/many_methods" => String::from("/many_methods"),
                    "/known" => format!("/{}", method.to_ascii_lowercase()),
                    route => route.replace("_", "/"),
                };

                let (addr, expected_res) = match stringify!($kind) {
                    "CLIENT" => {
                        ("httpbin.org:80", get_client_expected(&route))
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

pub fn get_client_expected(route: &str) -> Response {
    use rustnet::Status;
    use rustnet::header::names::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC,
        ACCESS_CONTROL_ALLOW_ORIGIN as ACAO, CONTENT_LENGTH, CONTENT_TYPE,
        LOCATION,
    };

//    let mut req = Request::new();
//    req.method = Method::from_str(method).unwrap();
//    req.headers.accept("*/*");
//    req.request_line.path = route.into();
//    req.headers.user_agent(DEFAULT_NAME);
//    req.headers.insert(HOST, "httpbin.org:80".into());

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
            res.status = Status(101u16);
        },
        "/status/201" => {
            res.status = Status(201u16);
        },
        "/status/301" => {
            res.headers.remove(&CONTENT_TYPE);
            res.status = Status(301u16);
            res.headers.insert(LOCATION, "/redirect/1".into());
        },
        "/status/404" => {
            res.status = Status(404u16);
        },
        "/status/502" => {
            res.status = Status(502u16);
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

pub fn get_server_expected(method: &str, route: &str) -> Response {
    use rustnet::{Status, DEFAULT_NAME};

    let mut res = Response::new();
    res.headers.server(DEFAULT_NAME);
    res.headers.cache_control("no-cache");
    res.headers.content_type("text/plain; charset=utf-8");

    match route {
        "/unknown" => {
            res.headers.content_length(482);
            res.status = Status(404u16);
            res.headers.content_type("text/html; charset=utf-8");
        },
        "/favicon.ico" => {
            res.headers.content_length(1406);
            res.headers.content_type("image/x-icon");
            res.headers.cache_control("max-age=604800");
        },
        "/about" => {
            res.headers.content_length(455);
            res.headers.content_type("text/html; charset=utf-8");
        },
        "/post" => {
            res.headers.content_length(575);
            res.status = Status(201u16);
            res.headers.content_type("text/html; charset=utf-8");
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
            res.headers.content_type("text/html; charset=utf-8");
        },
        "/many_methods" => match method {
            "HEAD" => res.headers.content_length(23),
            "POST" => {
                res.headers.content_length(23);
                res.status = Status(201u16);
            },
            "DELETE" => res.headers.content_length(25),
            "GET" | "PUT" => res.headers.content_length(22),
            "PATCH" | "TRACE" => res.headers.content_length(24),
            "OPTIONS" | "CONNECT" => res.headers.content_length(26),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }

    res
}
