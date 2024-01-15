#[cfg(test)]
#[macro_use]
mod common;

#[cfg(test)]
mod get {
    use super::*;
    run_test!(CLIENT: GET deny);
    run_test!(CLIENT: GET html);
    run_test!(CLIENT: GET json);
    run_test!(CLIENT: GET xml);
    run_test!(CLIENT: GET image_jpeg);
    run_test!(CLIENT: GET status_101);
    run_test!(CLIENT: GET status_200);
    run_test!(CLIENT: GET status_301);
    run_test!(CLIENT: GET status_404);
    run_test!(CLIENT: GET status_502);
}

#[cfg(test)]
mod post {
    use super::*;
    run_test!(CLIENT: POST status_201);
    run_test!(CLIENT: POST status_301);
    run_test!(CLIENT: POST status_404);
}

#[cfg(test)]
mod patch {
    use super::*;
    run_test!(CLIENT: PATCH status_200);
    run_test!(CLIENT: PATCH status_301);
    run_test!(CLIENT: PATCH status_404);
}

#[cfg(test)]
mod put {
    use super::*;
    run_test!(CLIENT: PUT status_200);
    run_test!(CLIENT: PUT status_301);
    run_test!(CLIENT: PUT status_404);
}

#[cfg(test)]
mod delete {
    use super::*;
    run_test!(CLIENT: DELETE status_200);
    run_test!(CLIENT: DELETE status_301);
    run_test!(CLIENT: DELETE status_404);
}

#[cfg(test)]
mod parse {
    use std::collections::VecDeque;
    use rustnet::{
        Body, Client, ClientCli, Headers, Method, Request, Style, StyleKind,
        StyleParts, Version,
    };
    use rustnet::headers::names::{
        ACCEPT, CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, HOST,
    };

    #[test]
    fn cli_args() {
        let mut args = VecDeque::from([
            "./client",
            "--plain",
            "--no-dates",
            "--output", "/Bs2Rhb1",
            "--method", "post",
            "-H", "acCEpT:*/*",
            "-H", "conteNt-leNgth:13",
            "-H", "caCHe-controL:no-cache",
            "--debug",
            "-H", "content-type:text/html; charset=utf-8",
            "-H", "pineaPPle:yum123",
            "--body", "This is a test meSSage :) in the request bOdy.",
            "httpbin.org/json"
        ]);

        let mut test_client = ClientCli::parse_args(&mut args).unwrap();

        let style = Style {
            req: StyleKind::Plain(StyleParts::LineBody),
            res: StyleKind::Plain(StyleParts::All)
        };

        let mut headers = Headers::new();
        headers.insert(ACCEPT, "*/*".into());
        headers.insert(CONTENT_LENGTH, 13.into());
        headers.insert(CACHE_CONTROL, "no-cache".into());
        headers.insert("Pineapple".into(), "yum123".into());
        headers.insert(CONTENT_TYPE, "text/html; charset=utf-8".into());
        let body_text = "This is a test meSSage :) in the request bOdy.";

        let expected_req = Request {
            method: Method::Post,
            path: "/json".into(),
            version: Version::default(),
            headers,
            body: Body::Text(body_text.into())
        };

        let mut expected_client = Client::builder()
            .do_debug(true)
            .no_dates(true)
            .style(style)
            .req(expected_req)
            .addr("httpbin.org:80")
            .build()
            .unwrap();

        if let Some(req) = test_client.req.as_mut() {
            req.headers.insert(HOST, "httpbin.org".into());
        }

        if let Some(req) = expected_client.req.as_mut() {
            req.headers.insert(HOST, "httpbin.org".into());
        }

        assert_eq!(test_client, expected_client);
    }
}
