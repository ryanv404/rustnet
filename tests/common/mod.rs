#![allow(unused_macros)]

use rustnet::Response;

// Start a test server in the background.
macro_rules! start_test_server {
    () => {
        #[test]
        fn start_test_server() {
            use std::process::{Command, Stdio};
            use rustnet::{utils, SERVER_NAME, TEST_SERVER_ADDR};

            let args = [
                "run", "--bin", SERVER_NAME, "--",
                "--test", "--", TEST_SERVER_ADDR
            ];

            let _ = Command::new("cargo")
                .args(&args[..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();

            if !utils::check_server(TEST_SERVER_ADDR) {
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
            use rustnet::{utils, CLIENT_NAME, TEST_SERVER_ADDR};

            let args = [
                "run", "--bin", CLIENT_NAME, "--",
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

            if utils::check_server(TEST_SERVER_ADDR) {
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
                use rustnet::{Body, Response, CLIENT_NAME, TEST_SERVER_ADDR};
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
                        let res = get_client_expected(method, &route);
                        ("httpbin.org:80", res)
                    },
                    "SERVER" => {
                        let res = get_server_expected(method, &route);
                        (TEST_SERVER_ADDR, res)
                    },
                    _ => unreachable!(),
                };

                let args = [
                    "run", "--bin", CLIENT_NAME, "--",
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

pub fn get_client_expected(method: &str, route: &str) -> Response {
    use std::str::FromStr;
    use rustnet::{Method, Request, Status};
    use rustnet::headers::names::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC,
        ACCESS_CONTROL_ALLOW_ORIGIN as ACAO, CONTENT_LENGTH, CONTENT_TYPE,
        HOST, LOCATION, SERVER,
    };

    let mut _req = Request::new();
    _req.method = Method::from_str(method).unwrap();
    _req.path = route.to_string().into();
    _req.headers.add_user_agent();
    _req.headers.add_accept("*/*");
    _req.headers.insert(HOST, "httpbin.org:80".into());

    let mut res = Response::new();
    res.headers.add_content_length(0);
    res.headers.insert(ACAO, "*".into());
    res.headers.insert(ACAC, "true".into());
    res.headers.add_connection("keep-alive");
    res.headers.insert(SERVER, "gunicorn/19.9.0".into());
    res.headers.add_content_type("text/html; charset=utf-8");

    match route {
        "/status/101" => {
            res.status = Status(101);
            res.headers.remove(&CONTENT_LENGTH);
            res.headers.add_connection("upgrade");
        },
        "/status/201" => {
            res.status = Status(201);
        },
        "/status/301" => {
            res.status = Status(301);
            res.headers.remove(&CONTENT_TYPE);
            res.headers.insert(LOCATION, "/redirect/1".into());
        },
        "/status/404" => {
            res.status = Status(404);
        },
        "/status/502" => {
            res.status = Status(502);
        },
        "/xml" => {
            res.headers.add_content_length(522);
            res.headers.add_content_type("application/xml");
        },
        "/json" => {
            res.headers.add_content_length(429);
            res.headers.add_content_type("application/json");
        },
        "/deny" => {
            res.headers.add_content_length(239);
            res.headers.add_content_type("text/plain");
        },
        "/html" => {
            res.headers.add_content_length(3741);
            res.headers.add_content_type("text/html; charset=utf-8");
        },
        "/image/jpeg" => {
            res.headers.add_content_length(35588);
            res.headers.add_content_type("image/jpeg");
        },
        _ => {},
    }

    res
}

pub fn get_server_expected(method: &str, route: &str) -> Response {
    use rustnet::Status;

    let mut res = Response::new();
    res.headers.add_server();
    res.headers.add_cache_control("no-cache");
    res.headers.add_content_type("text/plain; charset=utf-8");

    match route {
        "/unknown" => {
            res.headers.add_content_length(482);
            res.status = Status(404);
            res.headers.add_content_type("text/html; charset=utf-8");
        },
        "/favicon.ico" => {
            res.headers.add_content_length(1406);
            res.headers.add_content_type("image/x-icon");
            res.headers.add_cache_control("max-age=604800");
        },
        "/about" => {
            res.headers.add_content_length(455);
            res.headers.add_content_type("text/html; charset=utf-8");
        },
        "/post" => {
            res.status = Status(201);
            res.headers.add_content_length(575);
            res.headers.add_content_type("text/html; charset=utf-8");
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
            res.headers.add_content_length(575);
            res.headers.add_content_type("text/html; charset=utf-8");
        },
        "/many_methods" => match method {
            "HEAD" => res.headers.add_content_length(23),
            "POST" => {
                res.status = Status(201);
                res.headers.add_content_length(23);
            },
            "DELETE" => res.headers.add_content_length(25),
            "GET" | "PUT" => res.headers.add_content_length(22),
            "PATCH" | "TRACE" => res.headers.add_content_length(24),
            "OPTIONS" | "CONNECT" => res.headers.add_content_length(26),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }

    res
}
