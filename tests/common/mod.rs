// Start a test server in the background.
#[allow(unused_macros)]
macro_rules! start_test_server {
    () => {
        #[test]
        fn start_test_server() {
            use std::process::{Command, Stdio};
            use rustnet::{utils, TEST_SERVER_ADDR};

            let args = [
                "run", "--bin", "server", "--", "--test", "--",
                TEST_SERVER_ADDR
            ];

            let _ = Command::new("cargo")
                .args(&args[..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();

            if !utils::check_server_is_live(TEST_SERVER_ADDR) {
                assert!(false);
            }
        }
    };
}

// Shut down the test server using a shutdown route.
#[allow(unused_macros)]
macro_rules! shutdown_test_server {
    () => {
        #[test]
        fn shutdown_test_server() {
            use std::process::{Command, Stdio};
            use std::thread;
            use std::time::Duration;
            use rustnet::{utils, TEST_SERVER_ADDR};

            let args = [
                "run", "--bin", "client", "--", "--shutdown", "--",
                TEST_SERVER_ADDR
            ];

            let _ = Command::new("cargo")
                .args(&args[..])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();

            thread::sleep(Duration::from_millis(400));

            if utils::check_server_is_live(TEST_SERVER_ADDR) {
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

pub fn get_client_expected(method: &str, route: &str) -> rustnet::Response {
    use std::str::FromStr;
    use rustnet::{Method, Request, Response, Status, DEFAULT_NAME};
    use rustnet::headers::names::{
        ACCEPT, ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC,
        ACCESS_CONTROL_ALLOW_ORIGIN as ACAO, CONNECTION, CONTENT_LENGTH,
        CONTENT_TYPE, HOST, LOCATION, SERVER, USER_AGENT,
    };

    let mut _req = Request {
        method: Method::from_str(method).unwrap(),
        path: route.to_string().into(),
        ..Request::default()
    };
    _req.headers.insert(ACCEPT, "*/*".into());
    _req.headers.insert(HOST, "httpbin.org:80".into());
    _req.headers.insert(USER_AGENT, DEFAULT_NAME.into());

    let mut res = Response::default();
    res.headers.insert(ACAO, "*".into());
    res.headers.insert(ACAC, "true".into());
    res.headers.insert(CONTENT_LENGTH, 0.into());
    res.headers.insert(CONNECTION, "keep-alive".into());
    res.headers.insert(SERVER, "gunicorn/19.9.0".into());
    res.headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".into());

    match route {
        "/status/101" => {
            res.headers.remove(&CONTENT_LENGTH);
            res.status = Status::try_from(101u16).unwrap();
            res.headers.insert(CONNECTION, "upgrade".into());
        },
        "/status/201" => {
            res.status = Status::try_from(201u16).unwrap();
        },
        "/status/301" => {
            res.headers.remove(&CONTENT_TYPE);
            res.status = Status::try_from(301u16).unwrap();
            res.headers.insert(LOCATION, "/redirect/1".into());
        },
        "/status/404" => {
            res.status = Status::try_from(404u16).unwrap();
        },
        "/status/502" => {
            res.status = Status::try_from(502u16).unwrap();
        },
        "/xml" => {
            res.headers.insert(CONTENT_LENGTH, 522.into());
            res.headers.insert(CONTENT_TYPE, "application/xml".into());
        },
        "/json" => {
            res.headers.insert(CONTENT_LENGTH, 429.into());
            res.headers.insert(CONTENT_TYPE, "application/json".into());
        },
        "/deny" => {
            res.headers.insert(CONTENT_LENGTH, 239.into());
            res.headers.insert(CONTENT_TYPE, "text/plain".into());
        },
        "/html" => {
            res.headers.insert(CONTENT_LENGTH, 3741.into());
            res.headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".into());
        },
        "/image/jpeg" => {
            res.headers.insert(CONTENT_LENGTH, 35588.into());
            res.headers.insert(CONTENT_TYPE, "image/jpeg".into());
        },
        _ => {},
    }

    res
}

pub fn get_server_expected(method: &str, route: &str) -> rustnet::Response {
    use rustnet::{Response, Status, DEFAULT_NAME};
    use rustnet::headers::names::{
        CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, SERVER,
    };

    let mut res = Response::default();
    res.headers.insert(SERVER, DEFAULT_NAME.into());
    res.headers.insert(CACHE_CONTROL, "no-cache".into());
    res.headers.insert(CONTENT_TYPE, "text/plain; charset=utf-8".into());

    match route {
        "/unknown" => {
            res.status = Status::try_from(404u16).unwrap();
            res.headers.insert(CONTENT_LENGTH, 482.into());
            res.headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".into());
        },
        "/favicon.ico" => {
            res.headers.insert(CONTENT_LENGTH, 1406.into());
            res.headers.insert(CONTENT_TYPE, "image/x-icon".into());
            res.headers.insert(CACHE_CONTROL, "max-age=604800".into());
        },
        "/about" => {
            res.headers.insert(CONTENT_LENGTH, 455.into());
            res.headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".into());
        },
        "/post" => {
            res.status = Status::try_from(201u16).unwrap();
            res.headers.insert(CONTENT_LENGTH, 575.into());
            res.headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".into());
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
            res.headers.insert(CONTENT_LENGTH, 575.into());
            res.headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".into());
        },
        "/many_methods" => match method {
            "POST" => {
                res.status = Status::try_from(201u16).unwrap();
                res.headers.insert(CONTENT_LENGTH, 23.into());
            },
            "OPTIONS" | "CONNECT" => {
                res.headers.insert(CONTENT_LENGTH, 26.into())
            },
            "HEAD" => res.headers.insert(CONTENT_LENGTH, 23.into()),
            "DELETE" => res.headers.insert(CONTENT_LENGTH, 25.into()),
            "GET" | "PUT" => res.headers.insert(CONTENT_LENGTH, 22.into()),
            "PATCH" | "TRACE" => res.headers.insert(CONTENT_LENGTH, 24.into()),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }

    res
}
