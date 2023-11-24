use librustnet::{Client, HeaderValue, Status, Version};
use librustnet::consts::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_ORIGIN, CONNECTION,
    CONTENT_LENGTH, CONTENT_TYPE, SERVER,
};

// Remote server responds with the status code corresponding to `code`.
macro_rules! test_by_status_code {
    ($label:ident: $(bad $code:literal),+) => {
        #[test]
        fn $label() {
            $(
                let mut client = Client::new()
                    .addr("httpbin.org:80")
                    .path(concat!("/status/", $code))
                    .send()
                    .unwrap();

                let res = client.recv().unwrap();

                assert_eq!(res.status_line.version, Version::OneDotOne);
                assert_eq!(res.status_line.status, Status(502));
                assert_eq!(res.get_header(&CONNECTION), Some(&HeaderValue::from("keep-alive")));
                assert_eq!(res.get_header(&CONTENT_LENGTH), Some(&HeaderValue::from("122")));
                assert_eq!(res.get_header(&CONTENT_TYPE), Some(&HeaderValue::from("text/html")));
                assert!(res.body().is_some());
            )+
        }
    };
    ($label:ident: $($code:literal),+) => {
        #[test]
        fn $label() {
            $(
                let mut client = Client::new()
                    .addr("httpbin.org:80")
                    .path(concat!("/status/", $code))
                    .send()
                    .unwrap();

                let res = client.recv().unwrap();

                assert_eq!(res.status_line.version, Version::OneDotOne);
                assert_eq!(res.status_line.status, Status($code));
                assert_eq!(res.get_header(&ACCESS_CONTROL_ALLOW_CREDENTIALS), Some(&HeaderValue::from("true")));
                assert_eq!(res.get_header(&ACCESS_CONTROL_ALLOW_ORIGIN), Some(&HeaderValue::from("*")));
                assert_eq!(res.get_header(&SERVER), Some(&HeaderValue::from("gunicorn/19.9.0")));
                assert_eq!(res.get_header(&CONNECTION), Some(&HeaderValue::from("keep-alive")));
                if !matches!($code, 100..=200) {
                    assert_eq!(res.get_header(&CONTENT_LENGTH), Some(&HeaderValue::from("0")));
                }
                assert_eq!(res.get_header(&CONTENT_TYPE), Some(&HeaderValue::from("text/html; charset=utf-8")));
                assert!(res.body().is_none());
            )+
        }
    };
}

test_by_status_code!(parse_1xx_status_response: 100, 102, 103);
test_by_status_code!(parse_2xx_status_response: 200, 201, 202);
test_by_status_code!(parse_3xx_status_response: 300, 306, 308);
test_by_status_code!(parse_4xx_status_response: 400, 404, 419);
test_by_status_code!(parse_5xx_status_response: 500, 501, 503);
test_by_status_code!(parse_invalid_status_response: bad 001, bad 1001);
