use std::net::TcpStream;

use librustnet::{
    Body, Headers, Method, NetReader, NetWriter, Request, RequestLine,
    Response, Status, StatusLine, Version,
};
use librustnet::consts::{
    CONTENT_LENGTH as CL, CONTENT_TYPE as CT, CONNECTION as CONN, DATE,
    LOCATION, WWW_AUTHENTICATE as WWW, X_MORE_INFO as XMORE, 
};

mod common;
use crate::common::get_expected_headers;

// Remote server responds with the status code corresponding to `code`.
macro_rules! get_responses {
    ($($code:literal),+) => {{
        let expected_headers = get_expected_headers();
        let stream = TcpStream::connect("httpbin.org:80").unwrap();

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

            let mut writer = NetWriter::from(stream.try_clone().unwrap());
            writer.send_request(&mut req).unwrap();

            let reader = NetReader::from(stream.try_clone().unwrap());
            let mut res = Response::recv(reader).unwrap();
            res.headers.remove(&DATE);

            exp.headers = expected_headers.get(&$code).cloned().unwrap();

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
                "\nFAILED at code {}:\ntest:\n{}\nexpected:\n{}\n",
                $code, res, exp);
        )+
    }};
}

#[test]
fn status_1xx_responses() {
    get_responses! [100, 101, 102, 103];
}

#[test]
fn status_2xx_responses() {
    get_responses! [200, 201, 202, 203, 204, 205, 206, 207, 208, 218, 226];
}

#[test]
fn status_3xx_responses() {
    get_responses! [300, 301, 302, 303, 304, 305, 306, 307, 308];
}

#[test]
fn status_4xx_responses() {
    get_responses! [
        400, 401, 402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412,
        413, 414, 415, 416, 417, 418, 419, 420, 421, 422, 423, 424, 425,
        426, 428, 429, 430, 431, 440, 444, 449, 450, 451, 460, 463, 464,
        494, 495, 496, 497, 498, 499
    ];
}

#[test]
fn status_5xx_responses() {
    get_responses! [
        500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 520,
        521, 522, 523, 524, 525, 526, 527, 529, 530, 561, 598, 599
    ];
}
