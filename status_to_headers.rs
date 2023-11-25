use std::collections::BTreeMap;

use librustnet::Headers;
use librustnet::consts::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_ORIGIN, SERVER, CONNECTION,
    LOCATION, CONTENT_LENGTH, CONTENT_TYPE, X_MORE_INFO, WWW_AUTHENTICATE,
};

fn main() {
    let mut expected: BTreeMap<u16, Headers> = BTreeMap::new();

    let default_headers = Headers(BTreeMap::from([
        (ACCESS_CONTROL_ALLOW_CREDENTIALS, "true".into()),
        (ACCESS_CONTROL_ALLOW_ORIGIN, "*".into()),
        (SERVER, "gunicorn/19.9.0".into()),
        (CONNECTION, "keep-alive".into()),
        (CONTENT_LENGTH, "0".into()),
        (CONTENT_TYPE, "text/html; charset=utf-8".into())
    ]));

    let status_codes: [u16; 94] = [
        100, 101, 102, 103, 200, 201, 202, 203, 204, 205, 206, 207, 208, 218, 226,
        300, 301, 302, 303, 304, 305, 306, 307, 308, 400, 401, 402, 403, 404, 405,
        406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 420,
        421, 422, 423, 424, 425, 426, 428, 429, 430, 431, 440, 444, 449, 450, 451,
        460, 463, 464, 494, 495, 496, 497, 498, 499, 500, 501, 502, 503, 504, 505,
        506, 507, 508, 509, 510, 511, 520, 521, 522, 523, 524, 525, 526, 527, 529,
        530, 561, 598, 599
    ];

    for code in &status_codes {
        expected.insert(*code, default_headers.clone());

        match *code {
            101 => {
                expected.entry(101)
                    .and_modify(|headers| {
                        headers.remove(&CONTENT_LENGTH);
                        headers.0.entry(CONNECTION)
                            .and_modify(|v| *v = "upgrade".into());
                    });
            },
            num @ (100 | 102 | 103 | 204) => {
                expected.entry(num)
                    .and_modify(|headers| headers.remove(&CONTENT_LENGTH));
            },
            num @ (301 | 302 | 303) => {
                expected.entry(num)
                    .and_modify(|headers| {
                        headers.remove(&CONTENT_TYPE);
                        headers.insert(LOCATION, "/redirect/1".into());
                    });
            },
            num @ 304 => {
                expected.entry(num)
                    .and_modify(|headers| {
                        headers.remove(&CONTENT_TYPE);
                        headers.remove(&CONTENT_LENGTH);
                    });
            },
            num @ (305 | 307) => {
                expected.entry(num)
                    .and_modify(|headers| {
                        headers.remove(&CONTENT_TYPE);
                        headers.insert(LOCATION, "/redirect/1".into());
                    });
            },
            401 => {
                expected.entry(401)
                    .and_modify(|headers| {
                        headers.remove(&CONTENT_TYPE);
                        headers.insert(WWW_AUTHENTICATE, r#"Basic realm="Fake Realm""#.into());
                    });
            },
            402 => {
                expected.entry(402)
                    .and_modify(|headers| {
                        headers.remove(&CONTENT_TYPE);
                        headers.insert(X_MORE_INFO, "http://vimeo.com/22053820".into());
                        headers.0.entry(CONTENT_LENGTH)
                            .and_modify(|v| *v = "17".into());
                    });
            },
            406 => {
                expected.entry(406)
                    .and_modify(|headers| {
                        headers.0.entry(CONTENT_LENGTH)
                            .and_modify(|v| *v = "142".into());
                        headers.0.entry(CONTENT_TYPE)
                            .and_modify(|v| *v = "application/json".into());
                    });
            },
            num @ (407 | 412) => {
                expected.entry(num)
                    .and_modify(|headers| headers.remove(&CONTENT_TYPE));
            },
            418 => {
                expected.entry(418)
                    .and_modify(|headers| {
                        headers.remove(&CONTENT_TYPE);
                        headers.0.entry(CONTENT_LENGTH)
                            .and_modify(|v| *v = "135".into());
                        headers.insert(X_MORE_INFO, "http://tools.ietf.org/html/rfc2324".into());
                    });
            },
            _ => {},
        }
    }
}
