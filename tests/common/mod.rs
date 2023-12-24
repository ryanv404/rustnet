#![allow(unused)]

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use rustnet::header_name::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC,
    ACCESS_CONTROL_ALLOW_ORIGIN as ACAO, CONNECTION as CONN,
    CONTENT_LENGTH as CL, CONTENT_TYPE as CT, HOST, LOCATION, SERVER,
    WWW_AUTHENTICATE as WWW, X_MORE_INFO as XMORE,
};
use rustnet::{
    Body, Connection, Header, Headers, Method, Request, RequestLine,
    Response, Status, StatusCode, StatusLine, Version,
};

macro_rules! run_server_tests {
    (START_TEST_SERVER) => {
        #[test]
        fn test_server_started() {
            let args = [
                "run", "--bin", "server", "--", "--test", "--",
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
    (SHUTDOWN_TEST_SERVER) => {
        #[test]
        fn test_server_shutdown() {
            let args = [
                "run", "--bin", "client", "--", "--shutdown", "--",
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
    (
        $label:ident:
        $( $method:literal, $uri_path:literal; )+
    ) => {
        #[test]
        fn $label() {
            $(
                let args = [
                    "run", "--bin", "client", "--", "--method", $method,
                    "--path", $uri_path, "--output", "sh", "--plain",
                    "--no-dates", "--", "127.0.0.1:7878"
                ];

                let output = Command::new("cargo").args(&args[..]).output()
                    .unwrap();
                let input = String::from_utf8(output.stdout.clone())
                    .unwrap();

                let test = get_test_output_server(&input);
                let expected = get_expected_output_server($method, $uri_path);

                assert_eq!(test, expected);
            )+
        }
    };
}

// Httpbin.org server responds with the status code corresponding to `$code`.
macro_rules! get_responses {
    ($($code:literal),+) => {
        let mut conn = TcpStream::connect("httpbin.org:80")
            .map_err(|_| NetError::NotConnected)
            .and_then(Connection::try_from)
            .unwrap();

        let mut req = Request {
            request_line: RequestLine {
                method: Method::Get,
                path: String::new(),
                version: Version::OneDotOne
            },
            headers: Headers::new(),
            body: Body::Empty
        };

        let mut expected = Response {
            status_line: StatusLine {
                status: Status(StatusCode(666)),
                version: Version::OneDotOne
            },
            headers: Headers::new(),
            body: Body::Empty
        };

        $(
            // Update the request and send it.
            let current_path = concat!("/status/", stringify!($code));
            req.request_line.path.clear();
            req.request_line.path.push_str(current_path);

            // Update the expected response.
            expected.status_line.status = Status(StatusCode($code));
            expected_headers($code, &mut expected);

            if let Err(e) = conn.send_request(&mut req) {
                panic!("Error sending request.\nCode {}.\n{e}", $code);
            }

            let Ok(mut res) = conn.recv_response() else {
                panic!("Error receiving response.\nCode {}.", $code);
            };

            if $code == 406 {
                assert!(res.body.is_json());
            }

            res.headers.remove(&DATE);
            res.body = Body::Empty;

            assert_eq!(res, expected);
        )+
    };
}

macro_rules! run_client_test {
    ($label:ident: $method:literal, $uri_path:literal) => {
        #[test]
        fn $label() {
            let args = [
                "run", "--bin", "client", "--", "--method", $method,
                "--path", $uri_path, "--output", "RHsh", "--plain",
                "--no-dates", "--", "httpbin.org:80"
            ];

            let (test_req, test_res) = Command::new("cargo")
                .args(&args[..])
                .output()
                .map(|output| {
                    let input = str::from_utf8(&output.stdout).unwrap();
                    get_test_output_client(input)
                })
                .unwrap();

            let exp_req = get_expected_req_client($method, $uri_path);
            let exp_res = get_expected_res_client($method, $uri_path);

            assert_eq!(test_req, exp_req);
            assert_eq!(test_res, exp_res);
        }
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

pub fn expected_headers(code: u16, expected: &mut Response) {
    // Clear prior headers.
    expected.headers.clear();

    // Add default expected headers.
    expected.headers.content_length(0);
    expected.headers.insert(ACAO, "*".into());
    expected.headers.connection("keep-alive");
    expected.headers.server("gunicorn/19.9.0");
    expected.headers.insert(ACAC, "true".into());
    expected.headers.content_type("text/html; charset=utf-8");

    // Update headers based on the status code.
    match code {
        101 => {
            expected.headers.remove(&CL);
            expected.headers.connection("upgrade");
        },
        100 | 102 | 103 | 204 => expected.headers.remove(&CL),
        301 | 302 | 303 | 305 | 307 => {
            expected.headers.remove(&CT);
            expected.headers.insert(LOCATION, "/redirect/1".into());
        },
        304 => {
            expected.headers.remove(&CT);
            expected.headers.remove(&CL);
        },
        401 => {
            expected.headers.remove(&CT);
            expected
                .headers
                .insert(WWW,
                    r#"Basic realm="Fake Realm""#.into());
        },
        402 => {
            expected.headers.remove(&CT);
            expected.headers.content_length(17);
            expected
                .headers
                .insert(XMORE,
                    "http://vimeo.com/22053820".into());
        },
        406 => {
            expected.headers.content_length(142);
            expected.headers.content_type("application/json");
        },
        407 => expected.headers.remove(&CT),
        418 => {
            expected.headers.remove(&CT);
            expected.headers.content_length(135);
            expected
                .headers
                .insert(XMORE,
                    "http://tools.ietf.org/html/rfc2324".into());
        },
        _ => {},
    }
}

pub fn favicon_route() -> Response {
    let mut headers = Headers::new();
    headers.content_length(1406);
    headers.server("rustnet/0.1");
    headers.content_type("image/x-icon");
    headers.cache_control("max-age=604800");

    Response {
        status_line: StatusLine {
            status: Status(StatusCode(200)),
            version: Version::OneDotOne
        },
        headers,
        body: Body::Empty
    }
}

pub fn many_methods(method: &str) -> Response {
    let status = if method == "POST" {
        Status(StatusCode(201))
    } else {
        Status(StatusCode(200))
    };

    let cont_len = match method {
        "DELETE" => 25,
        "GET" | "PUT" => 22,
        "HEAD" | "POST" => 23,
        "PATCH" | "TRACE" => 24,
        "OPTIONS" | "CONNECT" => 26,
        _ => unreachable!(),
    };

    let mut headers = Headers::new();
    headers.server("rustnet/0.1");
    headers.content_length(cont_len);
    headers.cache_control("no-cache");
    headers.content_type("text/plain; charset=utf-8");

    Response {
        status_line: StatusLine {
            status,
            version: Version::OneDotOne
        },
        headers,
        body: Body::Empty
    }
}

pub fn unknown_route(method: &str) -> Response {
    let mut headers = Headers::new();
    headers.content_length(482);
    headers.server("rustnet/0.1");
    headers.cache_control("no-cache");
    headers.content_type("text/html; charset=utf-8");

    Response {
        status_line: StatusLine {
            status: Status(StatusCode(404)),
            version: Version::OneDotOne
        },
        headers,
        body: Body::Empty
    }
}

pub fn known_route(method: &str) -> Response {
    let status = if method == "POST" {
        Status(StatusCode(201))
    } else {
        Status(StatusCode(200))
    };

    let mut headers = Headers::new();
    headers.content_length(575);
    headers.server("rustnet/0.1");
    headers.cache_control("no-cache");
    headers.content_type("text/html; charset=utf-8");

    Response {
        status_line: StatusLine {
            status,
            version: Version::OneDotOne
        },
        headers,
        body: Body::Empty
    }
}

pub fn get_expected_req_client(
    method_str: &str,
    path_str: &str
) -> Request {
    let mut req_headers = Headers::new();
    req_headers.accept("*/*");
    req_headers.user_agent("rustnet/0.1");
    req_headers.insert(HOST, "httpbin.org:80".into());

    let method = Method::from_str(method_str).unwrap();

    Request {
        request_line: RequestLine {
            method,
            path: path_str.to_string(),
            version: Version::OneDotOne
        },
        headers: req_headers,
        body: Body::Empty
    }
}

pub fn get_expected_res_client(
    method_str: &str,
    path_str: &str
) -> Response {
    let status = if path_str == "/status/201" {
        Status(StatusCode(201))
    } else if path_str == "/status/203" {
        Status(StatusCode(203))
    } else {
        Status(StatusCode(200))
    };

    let mut res_headers = Headers::new();
    res_headers.insert(ACAO, "*".into());
    res_headers.connection("keep-alive");
    res_headers.server("gunicorn/19.9.0");
    res_headers.insert(ACAC, "true".into());

    let text = "text/plain";
    let jpeg = "image/jpeg";
    let xml = "application/xml";
    let json = "application/json";
    let html = "text/html; charset=utf-8";

    match path_str {
        "/xml" => {
            res_headers.content_length(522);
            res_headers.content_type(xml);
        },
        "/json" => {
            res_headers.content_length(429);
            res_headers.content_type(json);
        },
        "/deny" => {
            res_headers.content_length(239);
            res_headers.content_type(text);
        },
        "/html" => {
            res_headers.content_length(3741);
            res_headers.content_type(html);
        },
        "/status/200" | "/status/201" | "/status/203" => {
            res_headers.content_length(0);
            res_headers.content_type(html);
        },
        "/robots.txt" => {
            res_headers.content_length(30);
            res_headers.content_type(text);
        },
        "/image/jpeg" => {
            res_headers.content_length(35588);
            res_headers.content_type(jpeg);
        },
        "/encoding/utf8" => {
            res_headers.content_length(14239);
            res_headers.content_type(html);
        },
        _ => unreachable!(),
    }

    Response {
        status_line: StatusLine {
            status,
            version: Version::OneDotOne
        },
        headers: res_headers,
        body: Body::Empty
    }
}

pub fn get_expected_output_server(method: &str, path: &str) -> Response {
    match path {
        "/foo" => unknown_route(method),
        "/favicon.ico" => favicon_route(),
        "/many_methods" => many_methods(method),
        "/"
            | "/head"
            | "/post"
            | "/put"
            | "/patch"
            | "/delete"
            | "/trace"
            | "/options"
            | "/connect" => known_route(method),
        _ => unreachable!(),
    }
}

pub fn get_test_output_client(input: &str) -> (Request, Response) {
    let mut req = Request::default();
    let mut res = Response::default();

    let mut lines = input.trim_start().lines();

    let Some(req_line) = lines.next() else {
        panic!("Can't parse client test req line.");
    };

    let Ok(request_line) = RequestLine::from_str(req_line.trim()) else {
        panic!("Can't parse client test req line.");
    };

    req.request_line = request_line;

    while let Some(line) = lines.next() {
        let trim = line.trim();

        if trim.is_empty() {
            break;
        }

        if let Err(e) = req.headers.insert_parsed_header_str(trim) {
            panic!("Can't parse client test req headers.");
        }
    }

    req.headers.insert(HOST, "httpbin.org:80".into());

    while let Some(line) = lines.next() {
        let trim = line.trim();

        if trim.starts_with("HTTP") {
            let Ok(status_line) = StatusLine::from_str(trim) else {
                panic!("Can't parse client test req line.");
            };

            res.status_line = status_line;
            break;
        }
    }

    while let Some(line) = lines.next() {
        let trim = line.trim();

        if trim.is_empty() {
            break;
        }

        if let Err(e) = res.headers.insert_parsed_header_str(trim) {
            panic!("Can't parse client test res headers.");
        }
    }

    (req, res)
}

pub fn get_test_output_server(input: &str) -> Response {
    let mut res = Response::default();
    let mut lines = input.trim_start().lines();

    let Some(stat_line) = lines.next() else {
        panic!("Can't parse server test status line 1.");
    };

    let Ok(status_line) = StatusLine::from_str(stat_line.trim()) else {
        panic!("Can't parse server test status line 2.");
    };

    res.status_line = status_line;

    while let Some(line) = lines.next() {
        let trim = line.trim();

        if trim.is_empty() {
            break;
        }

        if let Err(e) = res.headers.insert_parsed_header_str(trim) {
            panic!("Can't parse client test res headers.");
        }
    }

    res
}
