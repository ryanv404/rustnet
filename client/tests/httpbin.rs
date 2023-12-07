use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::get_expected_headers;

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

macro_rules! run_client_test {
    ($label:ident: $method:literal, $uri_path:literal) => {
        #[test]
        fn $label() {
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

            let output = get_trimmed_test_output(&output.stdout);

            let filename = format!(
                "{}_{}.txt",
                $method.to_lowercase(),
                stringify!($label)
            );

            let exp_file: PathBuf = [
                ROOT,
                "test_data",
                &filename
            ].iter().collect();

            let expected = get_expected_from_file(&exp_file);

            assert_eq!(output, expected, "Failure at: {}", stringify!($label));
        }
    };
}

fn get_trimmed_test_output(output: &[u8]) -> String {
    let output_str = String::from_utf8_lossy(output);

    output_str
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

fn get_expected_from_file(exp_file: &Path) -> String {
    let mut exp_output = String::new();

    let mut f = File::open(exp_file).unwrap();
    f.read_to_string(&mut exp_output).unwrap();

    exp_output
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

// Client tests
mod get {
    use super::*;
    use std::net::TcpStream;
    use librustnet::{
        Body, Headers, Method, NetReader, NetWriter, Request, RequestLine,
        Response, Status, StatusLine, Version,
    };
    use librustnet::consts::{
        CONTENT_LENGTH as CL, CONTENT_TYPE as CT, CONNECTION as CONN, DATE,
        LOCATION, WWW_AUTHENTICATE as WWW, X_MORE_INFO as XMORE, 
    };

    run_client_test!(deny: "GET", "/deny");
    run_client_test!(html: "GET", "/html");
    run_client_test!(json: "GET", "/json");
    run_client_test!(xml: "GET", "/xml");
    run_client_test!(robots_txt: "GET", "/robots.txt");
    run_client_test!(encoding_utf8: "GET", "/encoding/utf8");
    run_client_test!(image_jpeg: "GET", "/image/jpeg");
    run_client_test!(image_png: "GET", "/image/png");
    run_client_test!(image_svg: "GET", "/image/svg");
    run_client_test!(image_webp: "GET", "/image/webp");

    // Remote server responds with the status code corresponding to `code`.
    macro_rules! get_responses {
        ($($code:literal),+) => {{
            let expected_headers = get_expected_headers();

            let Ok(stream) = TcpStream::connect("httpbin.org:80") else {
                panic!(
                    "Could not connect to remote host in: {}",
                    stringify!($label)
                );
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
                    Err(e) => panic!(
                        "Error while receiving response at code: {}\n{e}",
                        $code
                    ),
                };

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

                assert_eq!(res, exp,
                    "\nFAILED at code {}:\n\
                    test:\n{}\n\
                    expected:\n{}\n",
                    $code, res, exp);
            )+
        }};
    }

    #[test]
    fn status_1xx() {
        get_responses! [100, 101, 102, 103];
    }

    #[test]
    fn status_2xx() {
        get_responses! [
            200, 201, 202, 203, 204, 205, 206, 207, 208, 218, 226
        ];
    }

    #[test]
    fn status_3xx() {
        get_responses! [300, 301, 302, 303, 304, 305, 306, 307, 308];
    }

    #[test]
    fn status_4xx() {
        get_responses! [
            400, 401, 402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412,
            413, 414, 415, 416, 417, 418, 419, 420, 421, 422, 423, 424, 425,
            426, 428, 429, 430, 431, 440, 444, 449, 450, 451, 460, 463, 464,
            494, 495, 496, 497, 498, 499
        ];
    }

    #[test]
    fn status_5xx() {
        get_responses! [
            500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 520,
            521, 522, 523, 524, 525, 526, 527, 529, 530, 561, 598, 599
        ];
    }
}

mod post {
    use super::*;

    run_client_test!(status_201: "POST", "/status/201");
}

mod patch {
    use super::*;

    run_client_test!(status_201: "PATCH", "/status/201");
}

mod put {
    use super::*;

    run_client_test!(status_203: "PUT", "/status/203");
}

mod delete {
    use super::*;

    run_client_test!(status_200: "DELETE", "/status/200");
}
