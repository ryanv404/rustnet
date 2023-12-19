#![allow(unused)]

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

use rustnet::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC, ACCESS_CONTROL_ALLOW_ORIGIN as ACAO,
    CONNECTION as CONN, CONTENT_LENGTH as CL, CONTENT_TYPE as CT, LOCATION, SERVER,
    WWW_AUTHENTICATE as WWW, X_MORE_INFO as XMORE,
};
use rustnet::{Headers, Response};

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
                    "127.0.0.1:7878"
                ])
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
                    "127.0.0.1:7878"
                ])
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
                        "127.0.0.1:7878"
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

pub fn server_is_live(is_shutting_down: bool) -> bool {
    let timeout = Duration::from_millis(200);
    let socket = SocketAddr::new(
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        7878);

    for _ in 0..5 {
        if TcpStream::connect_timeout(&socket, timeout).is_ok() {
            if !is_shutting_down {
                return true;
            } else {
                thread::sleep(timeout);
            }
        } else {
            if is_shutting_down {
                return false;
            } else {
                thread::sleep(timeout);
            }
        }
    }

    if is_shutting_down {
        true
    } else {
        false
    }
}

pub fn add_expected_headers(code: u16, expected: &mut Response) {
    // Clear prior headers.
    expected.headers.clear();

    // Add default headers.
    expected.headers.insert(CL, b"0"[..].into());
    expected.headers.insert(ACAO, b"*"[..].into());
    expected.headers.insert(ACAC, b"true"[..].into());
    expected.headers.insert(CONN, b"keep-alive"[..].into());
    expected.headers.insert(SERVER, b"gunicorn/19.9.0"[..].into());
    expected.headers.insert(CT, b"text/html; charset=utf-8"[..].into());

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

pub fn favicon_route_output() -> String {
    "\
HTTP/1.1 200 OK
Cache-Control: max-age=604800
Content-Length: 1406
Content-Type: image/x-icon
Server: rustnet/0.1.0".to_string()
}

pub fn many_methods_output(method: &str) -> String {
    let status_200 = "200 OK";
    let status_201 = "201 Created";

    let (status, len) = match method {
        "GET" => (status_200, 22),
        "HEAD" => (status_200, 23),
        "POST" => (status_201, 23),
        "PUT" => (status_200, 22),
        "PATCH" => (status_200, 24),
        "DELETE" => (status_200, 25),
        "TRACE" => (status_200, 24),
        "OPTIONS" => (status_200, 26),
        "CONNECT" => (status_200, 26),
        _ => unreachable!(),
    };

    let mut output = format!("\
HTTP/1.1 {status}
Cache-Control: no-cache
Content-Length: {len}
Content-Type: text/plain; charset=utf-8
Server: rustnet/0.1.0");

    if method != "HEAD" {
        output.push_str(&format!("\nHi from the {method} route!"));
    }

    output
}

pub fn unknown_route_output(method: &str) -> String {
    if method == "HEAD" {
        "\
HTTP/1.1 404 Not Found
Cache-Control: no-cache
Content-Length: 482
Content-Type: text/html; charset=utf-8
Server: rustnet/0.1.0".to_string()
    } else {
        r#"HTTP/1.1 404 Not Found
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
</html>"#.to_string()
    }
}

pub fn known_route_output(method: &str) -> String {
    if method == "HEAD" {
        "\
HTTP/1.1 200 OK
Cache-Control: no-cache
Content-Length: 575
Content-Type: text/html; charset=utf-8
Server: rustnet/0.1.0".to_string()
    } else {
        format!(r#"HTTP/1.1 {}
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
</html>"#,
        if method == "POST" { "201 Created" } else { "200 OK" })
    }
}

pub fn get_expected_client_output(method: &str, path: &str) -> String {
    let jpeg = "image/jpeg";
    let text = "text/plain";
    let xml = "application/xml";
    let json = "application/json";
    let html = "text/html; charset=utf-8";
    let status_200 = "200 OK";
    let status_201 = "201 Created";
    let status_203 = "203 Non-Authoritative Information";

    let (status, len, contype) = match path {
        "/xml" => (status_200, 522, xml),
        "/json" => (status_200, 429, json),
        "/deny" => (status_200, 239, text),
        "/html" => (status_200, 3741, html),
        "/status/200" => (status_200, 0, html),
        "/status/201" => (status_201, 0, html),
        "/status/203" => (status_203, 0, html),
        "/robots.txt" => (status_200, 30, text),
        "/image/jpeg" => (status_200, 35588, jpeg),
        "/encoding/utf8" => (status_200, 14239, html),
        _ => unreachable!(),
    };

    format!("\
{method} {path} HTTP/1.1
Accept: */*
Content-Length: 0
Host: httpbin.org:80
User-Agent: rustnet/0.1.0
HTTP/1.1 {status}
Access-Control-Allow-Credentials: true
Access-Control-Allow-Origin: *
Connection: keep-alive
Content-Length: {len}
Content-Type: {contype}
Server: gunicorn/19.9.0")
}

pub fn get_expected_server_output(method: &str, path: &str) -> String {
    match path {
        "/foo" => unknown_route_output(method),
        "/favicon.ico" => favicon_route_output(),
        "/many_methods" => many_methods_output(method),
        "/" | "/head" | "/post" | "/put" | "/patch" | "/delete" | "/trace" |
            "/options" | "/connect" => known_route_output(method),
        _ => unreachable!(),
    }
}

pub fn get_client_test_output(input: &str) -> String {
    let mut output = input
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                None
            } else if line.starts_with("Host:") {
                Some("Host: httpbin.org:80")
            } else {
                Some(line)
            }
        })
        .fold(String::new(), |mut acc, s| {
            acc.push_str(s);
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
            acc.push_str(line);
            acc.push('\n');
            acc
        });

    output.pop();
    output
}
