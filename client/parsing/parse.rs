use std::collections::BTreeMap;

use librustnet::{
    Client, Headers, HeaderValue, Status, Version, get_expected_headers,
};
use librustnet::consts::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC, CONTENT_LENGTH as CL, REDIRECT,
    CONTENT_TYPE as CT, ACCESS_CONTROL_ALLOW_ORIGIN as ACAO, SERVER, LOCATION,
    CONNECTION, WWW_AUTHENTICATE as WWW, X_MORE_INFO as XMORE, 
};

mod common;

// Remote server responds with the status code corresponding to `code`.
#[ignore]
#[test]
fn responses_1xx() {
    let expected = get_expected_headers();
}

#[ignore]
#[test]
fn responses_2xx() {
    let expected = get_expected_headers();
}

#[ignore]
#[test]
fn responses_3xx() {
    let expected = get_expected_headers();
}

#[ignore]
#[test]
fn responses_4xx() {
    let expected = get_expected_headers();
}

#[ignore]
#[test]
fn responses_5xx() {


}
    macro_rules! get_responses {
        ([ $($code:literal),+ ]) => {{
            let expected = get_expected_headers();
            let mut client = Client::builder().addr("httpbin.org:80")
                .path("/status/200").build().unwrap();


            $(
                let Ok(req) = client.req.as_mut() else {
                    panic!("responses_5xx FAILED as code: {}", $code);
                };

                *req.status_line.path = format!("/status/{}", $code);

                client.send().unwrap();
                client.recv(&mut cloned_conn).unwrap();

            let res = client.res.take().unwrap();

            assert_eq!(res.status_line.version, Version::OneDotOne);
            assert_eq!(res.status_line.status, Status($code));
            assert_eq!(
                res.headers.get(&ACAC),
                Some(&HeaderValue(Vec::from("true")))
            );
            assert_eq!(
                res.headers.get(&ACAO),
                Some(&HeaderValue(Vec::from("*")))
            );
            assert_eq!(
                res.headers.get(&SERVER),
                Some(&HeaderValue(Vec::from("gunicorn/19.9.0")))
            );
            assert_eq!(
                res.headers.get(&CONNECTION),
                Some(&HeaderValue(Vec::from("keep-alive")))
            );
            if !matches!($code, 100..=200) {
                assert_eq!(
                    res.headers.get(&CL),
                    Some(&HeaderValue(Vec::from("0")))
                );
            }
            assert_eq!(
                res.headers.get(&CT),
                Some(&HeaderValue(Vec::from("text/html; charset=utf-8")))
            );
            assert!(res.body().is_empty());
        }
    }};
}

get_1xx_responses! [100, 101, 102, 103]
get_2xx_responses! [200, 201, 202, 203, 204, 205, 206, 207, 208, 218, 226]
get_3xx_responses! [300, 301, 302, 303, 304, 305, 306, 307, 308]
get_4xx_responses! [
    400, 401, 402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414,
    415, 416, 417, 418, 419, 420, 421, 422, 423, 424, 425, 426, 428, 429, 430,
    431, 440, 444, 449, 450, 451, 460, 463, 464, 494, 495, 496, 497, 498, 499
]
get_5xx_responses! [
    500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 520, 521, 522,
    523, 524, 525, 526, 527, 529, 530, 561, 598, 599
]

//get_expected_headers()
