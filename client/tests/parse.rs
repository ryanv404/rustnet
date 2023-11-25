// Remote server responds with the status code corresponding to `code`.
macro_rules! get_responses_by_status_code {
    ($( $name:ident: $($code:literal),+; )+) => {
        $(
            #[test]
            fn $name() {
                $(
                    let mut client = Client::new()
                        .addr("httpbin.org:80")
                        .path(concat!("/status/", $code))
                        .send()
                        .unwrap();
                    let res = client.recv().unwrap();
                    assert_eq!(res.status_line.version, Version::OneDotOne);
                    assert_eq!(res.status_line.status, Status($code));
                    assert_eq!(
                        res.get_header(&ACCESS_CONTROL_ALLOW_CREDENTIALS),
                        Some(&HeaderValue::from("true"))
                    );
                    assert_eq!(
                        res.get_header(&ACCESS_CONTROL_ALLOW_ORIGIN),
                        Some(&HeaderValue::from("*"))
                    );
                    assert_eq!(
                        res.get_header(&SERVER),
                        Some(&HeaderValue::from("gunicorn/19.9.0"))
                    );
                    assert_eq!(
                        res.get_header(&CONNECTION),
                        Some(&HeaderValue::from("keep-alive"))
                    );
                    if !matches!($code, 100..=200) {
                        assert_eq!(
                            res.get_header(&CONTENT_LENGTH),
                            Some(&HeaderValue::from("0"))
                        );
                    }
                    assert_eq!(
                        res.get_header(&CONTENT_TYPE),
                        Some(&HeaderValue::from("text/html; charset=utf-8"))
                    );
                    assert!(res.body().is_none());
                )+
            }
        )+
    };
}

mod response {
    use librustnet::{Client, HeaderValue, Status, Version};
    use librustnet::consts::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_ORIGIN,
        CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, SERVER,
    };

    get_responses_by_status_code!{
        status_1xx: 100, 102, 103;
        status_2xx: 200, 201, 202;
        status_3xx: 300, 306, 308;
        status_4xx: 400, 404, 419;
        status_5xx: 500, 501, 503;
    }
}
