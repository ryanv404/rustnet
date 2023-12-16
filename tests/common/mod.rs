#![allow(unused)]

use std::collections::BTreeMap;
use std::net::TcpStream;

use rustnet::consts::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC, ACCESS_CONTROL_ALLOW_ORIGIN as ACAO,
    CONNECTION as CONN, CONTENT_LENGTH as CL, CONTENT_TYPE as CT, LOCATION, SERVER,
    WWW_AUTHENTICATE as WWW, X_MORE_INFO as XMORE,
};
use rustnet::{Headers, Response};

pub const LOCAL_ADDR: &str = "127.0.0.1:7878";

pub const CONNECT_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 26
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the CONNECT route!";

pub const DELETE_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 25
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the DELETE route!";

pub const GET_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 22
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the GET route!";

pub const KNOWN_ROUTE: &str = r#"
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 575
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0

    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <meta charset="utf-8">
            <title>Home</title>
        </head>
        <body style="background-color:black;">
            <main style="color:white;">
                <h1 style="text-align:center; padding:10px;">Welcome home.</h1>
                <h2>Links:</h2>
                <ul style="list-style-type:none;">
                    <li><a href="/about" style="color:lightgray; text-decoration:none;">About</a></li>
                </ul>
            </main>
        </body>
    </html>"#;

pub const UNKNOWN_ROUTE: &str = r#"
    HTTP/1.1 404 Not Found
    Cache-Control: no-cache
    Content-Length: 482
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0

    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta charset="utf-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <title>Not Found</title>
        </head>
        <body style="background-color:black;">
            <main style="color:white;">
                <h2 style="text-align:left;">Sorry, that page could not be found.</h2>
                <p><a href="/" style="color:lightgray; text-decoration:none;">Home</a></p>
            </main>
        </body>
    </html>"#;

pub const HEAD_KNOWN_ROUTE: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 575
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0";

pub const HEAD_UNKNOWN_ROUTE: &str = "\
    HTTP/1.1 404 Not Found
    Cache-Control: no-cache
    Content-Length: 482
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0";

pub const HEAD_FAVICON: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: max-age=604800
    Content-Length: 1406
    Content-Type: image/x-icon
    Server: rustnet/0.1.0";

pub const HEAD_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 23
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0";

pub const OPTIONS_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 26
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the OPTIONS route!";

pub const PATCH_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 24
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the PATCH route!";

pub const POST_KNOWN_ROUTE: &str = r#"
    HTTP/1.1 201 Created
    Cache-Control: no-cache
    Content-Length: 575
    Content-Type: text/html; charset=utf-8
    Server: rustnet/0.1.0

    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <meta charset="utf-8">
            <title>Home</title>
        </head>
        <body style="background-color:black;">
            <main style="color:white;">
                <h1 style="text-align:center; padding:10px;">Welcome home.</h1>
                <h2>Links:</h2>
                <ul style="list-style-type:none;">
                    <li><a href="/about" style="color:lightgray; text-decoration:none;">About</a></li>
                </ul>
            </main>
        </body>
    </html>"#;

pub const POST_MANY_METHODS: &str = "\
    HTTP/1.1 201 Created
    Cache-Control: no-cache
    Content-Length: 23
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the POST route!";

pub const PUT_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 22
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the PUT route!";

pub const TRACE_MANY_METHODS: &str = "\
    HTTP/1.1 200 OK
    Cache-Control: no-cache
    Content-Length: 24
    Content-Type: text/plain; charset=utf-8
    Server: rustnet/0.1.0

    Hi from the TRACE route!";

pub const DELETE_STATUS_200: &str = "\
    DELETE /status/200 HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 0
    Content-Type: text/html; charset=utf-8
    Server: gunicorn/19.9.0";

pub const GET_DENY: &str = "\
    GET /deny HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 239
    Content-Type: text/plain
    Server: gunicorn/19.9.0";

pub const GET_ENCODING_UTF8: &str = "\
    GET /encoding/utf8 HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 14239
    Content-Type: text/html; charset=utf-8
    Server: gunicorn/19.9.0";

pub const GET_HTML: &str = "\
    GET /html HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 3741
    Content-Type: text/html; charset=utf-8
    Server: gunicorn/19.9.0";

pub const GET_IMAGE_JPEG: &str = "\
    GET /image/jpeg HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 35588
    Content-Type: image/jpeg
    Server: gunicorn/19.9.0";

pub const GET_JSON: &str = "\
    GET /json HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 429
    Content-Type: application/json
    Server: gunicorn/19.9.0";

pub const GET_ROBOTS_TXT: &str = "\
    GET /robots.txt HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 30
    Content-Type: text/plain
    Server: gunicorn/19.9.0";

pub const GET_XML: &str = "\
    GET /xml HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 522
    Content-Type: application/xml
    Server: gunicorn/19.9.0";

pub const PATCH_STATUS_201: &str = "\
    PATCH /status/201 HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 201 Created
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 0
    Content-Type: text/html; charset=utf-8
    Server: gunicorn/19.9.0";

pub const POST_STATUS_201: &str = "\
    POST /status/201 HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 201 Created
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 0
    Content-Type: text/html; charset=utf-8
    Server: gunicorn/19.9.0";

pub const PUT_STATUS_203: &str = "\
    PUT /status/203 HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 203 Non-Authoritative Information
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 0
    Content-Type: text/html; charset=utf-8
    Server: gunicorn/19.9.0";

macro_rules! run_server_tests {
    (START_TEST_SERVER) => {
        #[test]
        fn test_server_started() {
            let _server = Command::new("cargo")
                .args([
                    "run",
                    "--bin",
                    "server",
                    "--",
                    "--shutdown",
                    "--",
                    LOCAL_ADDR
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();

            let mut attempt_num = 0;

            while attempt_num < 5 {
                if is_server_live(LOCAL_ADDR) {
                    return;
                } else {
                    thread::sleep(Duration::from_millis(300));
                    attempt_num += 1;
                }
            }

            panic!("Server took too long to go live.");
        }
    };
    (SHUTDOWN_TEST_SERVER) => {
        #[test]
        fn test_server_shutdown() {
            let _shutdown = Command::new("cargo")
                .args([
                    "run",
                    "--bin",
                    "client",
                    "--",
                    "--method",
                    "DELETE",
                    "--path",
                    "/__shutdown_server__",
                    "--server-tests",
                    "--",
                    LOCAL_ADDR
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();

            let mut attempt_num = 0;

            while attempt_num < 5 {
                if is_server_live(LOCAL_ADDR) {
                    thread::sleep(Duration::from_millis(200));
                    attempt_num += 1;
                } else {
                    break;
                }
            }

            assert!(!is_server_live(LOCAL_ADDR));
        }
    };
    ($( $label:ident: $method:literal, $uri_path:literal; )+) => {
        $(
            #[test]
            fn $label() {
                let output = Command::new("cargo")
                    .args([
                        "run",
                        "--bin",
                        "client",
                        "--",
                        "--method",
                        $method,
                        "--path",
                        $uri_path,
                        "--server-tests",
                        "--",
                        LOCAL_ADDR
                    ])
                    .output()
                    .unwrap();

                let output_str = String::from_utf8(output.stdout).unwrap();
                let output = get_server_test_output(&output_str);
                let expected = get_expected_server_output($method, $uri_path);

                assert_eq!(output, expected);
            }
        )+
    };
}

// Httpbin.org server responds with the status code corresponding to `$code`.
macro_rules! get_responses {
    ($($code:literal),+) => {
        let stream = TcpStream::connect("httpbin.org:80").unwrap();
        let mut reader = NetReader::from(stream.try_clone().unwrap());
        let mut writer = NetWriter::from(stream);

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
                status: Status(666),
                version: Version::OneDotOne
            },
            headers: Headers::new(),
            body: Body::Empty
        };

        $(
            // Update the request and send it.
            req.request_line.path.clear();
            req.request_line.path
                .push_str(concat!("/status/", stringify!($code)));

            if let Err(e) = writer.send_request(&mut req) {
                panic!("Error sending request.\nCode {}.\n{e}", $code);
            }

            // Update the expected response.
            expected.status_line.status = Status($code);
            add_expected_headers($code, &mut expected);

            // Get the test response.
            let mut res = match reader.recv_response() {
                Ok(res) => res,
                Err(e) => {
                    panic!("Error receiving response.\nCode {}.\n{e}", $code);
                },
            };

            if $code == 406 {
                assert!(res.body.is_json());
                res.body = Body::Empty;
            }

            res.headers.remove(&DATE);
            assert_eq!(res, expected);
        )+
    };
}

macro_rules! run_client_test {
    ($label:ident: $method:literal, $uri_path:literal) => {
        #[test]
        fn $label() {
            let output = Command::new("cargo")
                .args([
                    "run",
                    "--bin",
                    "client",
                    "--",
                    "--method",
                    $method,
                    "--path",
                    $uri_path,
                    "--client-tests",
                    "--",
                    "httpbin.org:80",
                ])
                .output()
                .unwrap();

            let output_str = String::from_utf8(output.stdout).unwrap();
            let output = get_client_test_output(&output_str);
            let expected = get_expected_client_output($method, $uri_path);

            assert_eq!(output, expected);
        }
    };
}

pub fn is_server_live(addr: &str) -> bool {
    TcpStream::connect(addr).is_ok()
}

pub fn add_expected_headers(code: u16, expected: &mut Response) {
    // Clear prior headers.
    expected.headers.clear();

    // Add default headers.
    expected.headers.insert(ACAC, b"true"[..].into());
    expected.headers.insert(ACAO, b"*"[..].into());
    expected
        .headers
        .insert(SERVER, b"gunicorn/19.9.0"[..].into());
    expected.headers.insert(CONN, b"keep-alive"[..].into());
    expected.headers.insert(CL, b"0"[..].into());
    expected
        .headers
        .insert(CT, b"text/html; charset=utf-8"[..].into());

    // Update headers based on the status code.
    match code {
        101 => {
            expected.headers.remove(&CL);
            expected
                .headers
                .entry(CONN)
                .and_modify(|v| *v = b"upgrade"[..].into());
        }
        100 | 102 | 103 | 204 => expected.headers.remove(&CL),
        301 | 302 | 303 | 305 | 307 => {
            expected.headers.remove(&CT);
            expected.headers.insert(LOCATION, b"/redirect/1"[..].into());
        }
        304 => {
            expected.headers.remove(&CT);
            expected.headers.remove(&CL);
        }
        401 => {
            expected.headers.remove(&CT);
            expected
                .headers
                .insert(WWW, br#"Basic realm="Fake Realm""#[..].into());
        }
        402 => {
            expected.headers.remove(&CT);
            expected
                .headers
                .insert(XMORE, b"http://vimeo.com/22053820"[..].into());
            expected
                .headers
                .entry(CL)
                .and_modify(|v| *v = b"17"[..].into());
        }
        406 => {
            expected
                .headers
                .entry(CL)
                .and_modify(|v| *v = b"142"[..].into());
            expected
                .headers
                .entry(CT)
                .and_modify(|v| *v = b"application/json"[..].into());
        }
        407 => expected.headers.remove(&CT),
        418 => {
            expected.headers.remove(&CT);
            expected
                .headers
                .entry(CL)
                .and_modify(|v| *v = b"135"[..].into());
            expected
                .headers
                .insert(XMORE, b"http://tools.ietf.org/html/rfc2324"[..].into());
        }
        _ => {}
    }
}

pub fn get_client_test_output(input: &str) -> String {
    let mut output = input
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            if line.starts_with("Host:") {
                if let Some((name, _old_value)) = line.split_once(':') {
                    Some(name)
                } else {
                    Some(line)
                }
            } else {
                Some(line)
            }
        })
        .fold(String::new(), |mut acc, s| {
           acc.push_str(&s);

            if s == "Host" {
                acc.push_str(": httpbin.org:80");
            }

            acc.push('\n');
            acc
        });

    output.pop();
    output
}

pub fn get_expected_client_output(method: &str, path: &str) -> String {
    let output = match method {
        "GET" => match path {
            "/deny" => GET_DENY,
            "/html" => GET_HTML,
            "/json" => GET_JSON,
            "/xml" => GET_XML,
            "/robots.txt" => GET_ROBOTS_TXT,
            "/encoding/utf8" => GET_ENCODING_UTF8,
            "/image/jpeg" => GET_IMAGE_JPEG,
            _ => unreachable!(),
        },
        "POST" => POST_STATUS_201,
        "PUT" => PUT_STATUS_203,
        "PATCH" => PATCH_STATUS_201,
        "DELETE" => DELETE_STATUS_200,
        _ => unreachable!(),
    };

    let mut output = output
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            if line.starts_with("Host:") {
                if let Some((name, _old_value)) = line.split_once(':') {
                    Some(name)
                } else {
                    Some(line)
                }
            } else {
                Some(line)
            }
        })
        .fold(String::new(), |mut acc, s| {
           acc.push_str(&s);

            if s == "Host" {
                acc.push_str(": httpbin.org:80");
            }

            acc.push('\n');
            acc
        });

    output.pop();
    output
}

pub fn get_expected_server_output(method: &str, path: &str) -> String {
    let output = match method {
        "GET" => match path {
            "/" => KNOWN_ROUTE,
            "/foo" => UNKNOWN_ROUTE,
            "/many_methods" => GET_MANY_METHODS,
            _ => unreachable!(),
        },
        "HEAD" => match path {
            "/head" => HEAD_KNOWN_ROUTE,
            "/foo" => HEAD_UNKNOWN_ROUTE,
            "/favicon.ico" => HEAD_FAVICON,
            "/many_methods" => HEAD_MANY_METHODS,
            _ => unreachable!(),
        },
        "POST" => match path {
            "/post" => POST_KNOWN_ROUTE,
            "/foo" => UNKNOWN_ROUTE,
            "/many_methods" => POST_MANY_METHODS,
            _ => unreachable!(),
        },
        "PUT" => match path {
            "/put" => KNOWN_ROUTE,
            "/foo" => UNKNOWN_ROUTE,
            "/many_methods" => PUT_MANY_METHODS,
            _ => unreachable!(),
        },
        "PATCH" => match path {
            "/patch" => KNOWN_ROUTE,
            "/foo" => UNKNOWN_ROUTE,
            "/many_methods" => PATCH_MANY_METHODS,
            _ => unreachable!(),
        },
        "DELETE" => match path {
            "/delete" => KNOWN_ROUTE,
            "/foo" => UNKNOWN_ROUTE,
            "/many_methods" => DELETE_MANY_METHODS,
            _ => unreachable!(),
        },
        "TRACE" => match path {
            "/trace" => KNOWN_ROUTE,
            "/foo" => UNKNOWN_ROUTE,
            "/many_methods" => TRACE_MANY_METHODS,
            _ => unreachable!(),
        },
        "OPTIONS" => match path {
            "/options" => KNOWN_ROUTE,
            "/foo" => UNKNOWN_ROUTE,
            "/many_methods" => OPTIONS_MANY_METHODS,
            _ => unreachable!(),
        },
        "CONNECT" => match path {
            "/connect" => KNOWN_ROUTE,
            "/foo" => UNKNOWN_ROUTE,
            "/many_methods" => CONNECT_MANY_METHODS,
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    let mut output = output
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() { None } else { Some(line) }
        })
        .fold(String::new(), |mut acc, line| {
            acc.push_str(&line);
            acc.push('\n');
            acc
        });

    output.pop();
    output
}

pub fn get_server_test_output(input: &str) -> String {
    let mut output = input
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() { None } else { Some(line) }
        })
        .fold(String::new(), |mut acc, line| {
            acc.push_str(&line);
            acc.push('\n');
            acc
        });

    output.pop();
    output
}
