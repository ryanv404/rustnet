use librustnet::{Client, Status, Version};

// Remote server responds with the status code corresponding to `code`.
macro_rules! test_by_status_code {
    ($label:ident: $(bad $code:literal),+) => {
        #[test]
        fn $label() {
            $(
                let mut client = Client::http()
                    .addr("httpbin.org:80")
                    .path(concat!("/status/", $code))
                    .send()
                    .unwrap();

                let res = client.recv().unwrap();
                assert_eq!(res.version, Version::OneDotOne);
                assert_eq!(res.status, Status(502));
            )+
        }
    };
    ($label:ident: $($code:literal),+) => {
        #[test]
        fn $label() {
            $(
                let mut client = Client::http()
                    .addr("httpbin.org:80")
                    .path(concat!("/status/", $code))
                    .send()
                    .unwrap();

                let res = client.recv().unwrap();
                assert_eq!(res.version, Version::OneDotOne);
                assert_eq!(res.status, Status($code));
            )+
        }
    };
}

test_by_status_code!(parse_1xx_status: 101, 102, 103);
test_by_status_code!(parse_2xx_status: 200, 201, 202);
test_by_status_code!(parse_3xx_status: 300, 301, 302);
test_by_status_code!(parse_4xx_status: 400, 403, 404);
test_by_status_code!(parse_5xx_status: 500, 501, 502);
test_by_status_code!(parse_bad_status: 999, 601, 701);
