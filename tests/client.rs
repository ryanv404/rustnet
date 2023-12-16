use std::net::TcpStream;
use std::process::Command;

use rustnet::consts::DATE;
use rustnet::{
    Body, Headers, Method, NetReader, NetWriter, Request, RequestLine, Response, Status,
    StatusLine, Version,
};

#[macro_use]
mod common;

use common::{add_expected_headers, get_client_test_output, get_expected_client_output};

mod get {
    use super::*;

    run_client_test!(deny: "GET", "/deny");
    run_client_test!(html: "GET", "/html");
    run_client_test!(json: "GET", "/json");
    run_client_test!(xml: "GET", "/xml");
    run_client_test!(robots_txt: "GET", "/robots.txt");
    run_client_test!(encoding_utf8: "GET", "/encoding/utf8");
    run_client_test!(image_jpeg: "GET", "/image/jpeg");

    #[test]
    fn status_1xx() {
        get_responses![100, 101, 102, 103];
    }

    #[test]
    fn status_2xx() {
        get_responses![200, 201, 202, 203, 204, 205, 206, 207, 208, 218];
    }

    #[test]
    fn status_3xx() {
        get_responses![300, 301, 302, 303, 304, 305, 306, 307, 308];
    }

    #[test]
    fn status_4xx() {
        get_responses![400, 401, 402, 404, 405, 406, 407, 408, 409, 418];
    }

    #[test]
    fn status_5xx() {
        get_responses![500, 501, 502, 503, 504, 505, 506, 507, 508, 509];
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
