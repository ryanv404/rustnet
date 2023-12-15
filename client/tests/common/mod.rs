use std::collections::BTreeMap;

use librustnet::Headers;

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

pub const GET_IMAGE_PNG: &str = "\
    GET /image/png HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 8090
    Content-Type: image/png
    Server: gunicorn/19.9.0";

pub const GET_IMAGE_SVG: &str = "\
    GET /image/svg HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 8984
    Content-Type: image/svg+xml
    Server: gunicorn/19.9.0";

pub const GET_IMAGE_WEBP: &str = "\
    GET /image/webp HTTP/1.1
    Accept: */*
    Content-Length: 0
    Host: 54.86.118.241:80
    User-Agent: rustnet/0.1.0

    HTTP/1.1 200 OK
    Access-Control-Allow-Credentials: true
    Access-Control-Allow-Origin: *
    Connection: keep-alive
    Content-Length: 10568
    Content-Type: image/webp
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

pub const VALID_STATUS_CODES: [u16; 94] = [
    100, 101, 102, 103, 200, 201, 202, 203, 204, 205, 206, 207, 208, 218, 226,
    300, 301, 302, 303, 304, 305, 306, 307, 308, 400, 401, 402, 403, 404, 405,
    406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 420,
    421, 422, 423, 424, 425, 426, 428, 429, 430, 431, 440, 444, 449, 450, 451,
    460, 463, 464, 494, 495, 496, 497, 498, 499, 500, 501, 502, 503, 504, 505,
    506, 507, 508, 509, 510, 511, 520, 521, 522, 523, 524, 525, 526, 527, 529,
    530, 561, 598, 599
];

pub fn get_expected_headers() -> BTreeMap<u16, Headers> {
    use librustnet::consts::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC,
        ACCESS_CONTROL_ALLOW_ORIGIN as ACAO,
        CONTENT_LENGTH as CL, CONTENT_TYPE as CT, CONNECTION as CONN,
        LOCATION, SERVER, WWW_AUTHENTICATE as WWW, X_MORE_INFO as XMORE, 
    };

    let mut expected = BTreeMap::<u16, Headers>::new();

    let default_headers = Headers(BTreeMap::from([
        (ACAC, "true".as_bytes().into()),
        (ACAO, "*".as_bytes().into()),
        (SERVER, "gunicorn/19.9.0".as_bytes().into()),
        (CONN, "keep-alive".as_bytes().into()),
        (CL, "0".as_bytes().into()),
        (CT, "text/html; charset=utf-8".as_bytes().into())
    ]));

    for code in &VALID_STATUS_CODES {
        expected.insert(*code, default_headers.clone());

        match *code {
            101 => {
                expected.entry(101)
                    .and_modify(|headers| {
                        headers.remove(&CL);
                        headers.0.entry(CONN).and_modify(
                            |v| *v = b"upgrade"[..].into());
                    });
            },
            num @ (100 | 102 | 103 | 204) => {
                expected.entry(num).and_modify(
                    |headers| headers.remove(&CL));
            },
            num @ (301 | 302 | 303 | 305 | 307) => {
                expected.entry(num)
                    .and_modify(|headers| {
                        headers.remove(&CT);
                        headers.insert(LOCATION,
                            b"/redirect/1"[..].into());
                    });
            },
            num @ 304 => {
                expected.entry(num)
                    .and_modify(|headers| {
                        headers.remove(&CT);
                        headers.remove(&CL);
                    });
            },
            401 => {
                expected.entry(401)
                    .and_modify(|headers| {
                        headers.remove(&CT);
                        headers.insert(WWW,
                            br#"Basic realm="Fake Realm""#[..].into());
                    });
            },
            402 => {
                expected.entry(402)
                    .and_modify(|headers| {
                        headers.remove(&CT);
                        headers.insert(XMORE,
                            b"http://vimeo.com/22053820"[..].into());
                        headers.0.entry(CL).and_modify(
                            |v| *v = b"17"[..].into());
                    });
            },
            406 => {
                expected.entry(406)
                    .and_modify(|headers| {
                        headers.0.entry(CL).and_modify(
                            |v| *v = b"142"[..].into());
                        headers.0.entry(CT).and_modify(
                            |v| *v = b"application/json"[..].into());
                    });
            },
            num @ (407 | 412) => {
                expected.entry(num).and_modify(
                    |headers| headers.remove(&CT));
            },
            418 => {
                expected.entry(418)
                    .and_modify(|headers| {
                        headers.remove(&CT);
                        headers.0.entry(CL).and_modify(
                            |v| *v = b"135"[..].into());
                        headers.insert(XMORE,
                            b"http://tools.ietf.org/html/rfc2324"[..].into());
                    });
            },
            _ => {},
        }
    }

    expected
}

// Remote server responds with the status code corresponding to `code`.
macro_rules! get_responses {
    ($($code:literal),+) => {
        use std::net::TcpStream;
        use std::thread::sleep;
        use std::time::Duration;
        use librustnet::{
            Body, Headers, Method, NetReader, NetWriter, Request, RequestLine,
            Response, Status, StatusLine, Version,
        };
        use librustnet::consts::{
            CONTENT_LENGTH as CL, CONTENT_TYPE as CT, CONNECTION as CONN,
            DATE, LOCATION, WWW_AUTHENTICATE as WWW, X_MORE_INFO as XMORE,
        };
        use $crate::common::get_expected_headers;

        let expected_headers = get_expected_headers();

        let Ok(stream) = TcpStream::connect("httpbin.org:80") else {
            panic!("Could not connect to remote host.");
        };

        let mut req = Request {
            request_line: RequestLine {
                method: Method::Get,
                path: String::new(),
                version: Version::OneDotOne
            },
            headers: Headers::new(),
            body: Body::Empty,
            reader: None
        };

        let mut exp = Response {
            status_line: StatusLine {
                status: Status(666),
                version: Version::OneDotOne
            },
            headers: Headers::new(),
            body: Body::Empty,
            writer: None
        };

        $(
            req.request_line.path = format!("/status/{}", $code);
            exp.status_line.status = Status($code);

            let mut writer = match stream.try_clone() {
                Ok(clone) => NetWriter::from(clone),
                Err(e) => {
                    panic!(
                        "Could not clone stream at status code: {}\n{e}",
                        $code
                    );
                },
            };

            match writer.send_request(&mut req) {
                Ok(_) => {},
                Err(e) => panic!(
                    "Error while sending request at code: {}\n{e}",
                    $code
                ),
            }

            let reader = match stream.try_clone() {
                Ok(clone) => NetReader::from(clone),
                Err(e) => panic!(
                    "Could not clone stream into NetReader \
                    at status code: {}\n{e}",
                    $code
                ),
            };

            let mut res = match Response::recv(reader) {
                Ok(res) => res,
                Err(_) => {
                    // Try again after a delay in case we are just
                    // rate-limited.
                    sleep(Duration::from_millis(100));

                    let reader = match stream.try_clone() {
                        Ok(clone) => NetReader::from(clone),
                        Err(e) => panic!(
                            "Could not clone stream into NetReader at status \
                            code {} during 2nd attempt to get a response.\n\
                            {e}",
                            $code
                        ),
                    };

                    match Response::recv(reader) {
                        Ok(res) => res,
                        Err(e) => panic!(
                            "Unable to get response for status code {} after \
                            2 attempts.\n{e}",
                            $code
                        ),
                    }
                }
            };

            // Remove dates in tests.
            res.headers.remove(&DATE);

            let Some(exp_headers) = expected_headers.get(&$code).cloned() else {
                panic!(
                    "Error while cloning expected headers at code: {}",
                    $code
                );
            };

            exp.headers = exp_headers;

            match $code {
                101 => {
                    exp.headers.remove(&CL);
                    exp.headers.entry(CONN)
                        .and_modify(|val| *val = b"upgrade"[..].into());
                },
                100 | 102 | 103 | 204 => {
                    exp.headers.remove(&CL);
                },
                301 | 302 | 303 | 305 | 307 => {
                    exp.headers.remove(&CT);
                    exp.headers.insert(LOCATION, b"/redirect/1"[..].into());
                },
                304 => {
                    exp.headers.remove(&CL);
                    exp.headers.remove(&CT);
                },
                401 => {
                    exp.headers.remove(&CT);
                    exp.headers.insert(WWW,
                        br#"Basic realm="Fake Realm""#[..].into());
                },
                402 => {
                    exp.headers.remove(&CT);
                    exp.headers.insert(XMORE,
                        b"http://vimeo.com/22053820"[..].into());
                },
                407 | 412 => {
                    exp.headers.remove(&CT);
                },
                418 => {
                    exp.headers.remove(&CT);
                    exp.headers.insert(XMORE,
                        b"http://tools.ietf.org/html/rfc2324"[..].into());
                },
                _ => {},
            }

            if $code == 406 {
                assert!(res.body.is_json());
                res.body = Body::Empty;
            }

            res.writer = None;
            exp.writer = None;

            assert_eq!(res, exp);
        )+
    };
}

macro_rules! run_client_test {
    ($label:ident: $method:literal, $uri_path:literal) => {
        #[test]
        fn $label() {
            use std::process::Command;
            use $crate::common::{get_expected_output, get_test_output};

            let output = Command::new("cargo")
                .args([
                    "run",
                    "-p", "client",
                    "--",
                    "--method", $method,
                    "--path", $uri_path,
                    "--client-tests",
                    "--",
                    "httpbin.org:80"
                ])
                .output()
                .unwrap();

            let output_str = String::from_utf8(output.stdout).unwrap();
            let output = get_test_output(&output_str);
            let expected = get_expected_output($method, $uri_path);

            assert_eq!(output, expected);
        }
    };
}

pub fn get_test_output(input: &str) -> String {
    input
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();

            if line.starts_with("Host:") {
                if let Some((name, _old_value)) = line.split_once(':') {
                    let current_host = format!("{name}: httpbin.org:80");
                    Some(current_host)
                } else {
                    Some(line.to_string())
                }
            } else if !line.is_empty() {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn get_expected_output(method: &str, path: &str) -> String {
    let output = match method {
        "GET" => match path {
            "/deny" => GET_DENY,
            "/html" => GET_HTML,
            "/json" => GET_JSON,
            "/xml" => GET_XML,
            "/robots.txt" => GET_ROBOTS_TXT,
            "/encoding/utf8" => GET_ENCODING_UTF8,
            "/image/jpeg" => GET_IMAGE_JPEG,
            "/image/png" => GET_IMAGE_PNG,
            "/image/svg" => GET_IMAGE_SVG,
            "/image/webp" => GET_IMAGE_WEBP,
            _ => unreachable!(),
        },
        "POST" => POST_STATUS_201,
        "PUT" => PUT_STATUS_203,
        "PATCH" => PATCH_STATUS_201,
        "DELETE" => DELETE_STATUS_200,
        _ => unreachable!(),
    };

    output
        .split('\n')
        .filter_map(|line| {
            let line = line.trim();

            if line.starts_with("Host:") {
                if let Some((name, _old_value)) = line.split_once(':') {
                    let current_host = format!("{name}: httpbin.org:80");
                    Some(current_host)
                } else {
                    Some(line.to_string())
                }
            } else if !line.is_empty() {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}
