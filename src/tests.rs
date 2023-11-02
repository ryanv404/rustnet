#[cfg(test)]
mod std_headers {
    use crate::HeaderName;
    use crate::header::names::TEST_HEADERS;

    #[test]
    fn from_lowercase_bytes() {
        TEST_HEADERS.iter().for_each(|&(std, bytes)| {
            let std_hdr = HeaderName::from(std);
            let parsed_hdr = HeaderName::try_from(bytes).unwrap();
            assert_eq!(std_hdr, parsed_hdr);
        });
    }

    #[test]
    fn from_uppercase_bytes() {
        TEST_HEADERS.iter().for_each(|&(std, bytes)| {
            let std_hdr = HeaderName::from(std);
            let parsed_hdr = HeaderName::try_from(bytes
                .to_ascii_uppercase().as_slice()).unwrap();
            assert_eq!(std_hdr, parsed_hdr);
        });
    }
}

#[cfg(test)]
mod http_method {
    use crate::{Method, Request};

    #[test]
    fn from_str_request_line() {
        let get = "GET /test HTTP/1.1";
        let put = "PUT /test HTTP/1.1";
        let post = "POST /test HTTP/1.1";
        let head = "HEAD /test HTTP/1.1";
        let patch = "PATCH /test HTTP/1.1";
        let trace = "TRACE /test HTTP/1.1";
        let delete = "DELETE /test HTTP/1.1    ";
        let options = "OPTIONS /test HTTP/1.1";
        let connect = "CONNECT /test HTTP/1.1";
        let bad1 = "GET";
        let bad2 = "GET /test";
        let bad3 = "FOO bar baz";
        assert_eq!(Request::parse_request_line(get).unwrap().0, Method::Get);
        assert_eq!(Request::parse_request_line(put).unwrap().0, Method::Put);
        assert_eq!(Request::parse_request_line(post).unwrap().0, Method::Post);
        assert_eq!(Request::parse_request_line(head).unwrap().0, Method::Head);
        assert_eq!(Request::parse_request_line(patch).unwrap().0, Method::Patch);
        assert_eq!(Request::parse_request_line(trace).unwrap().0, Method::Trace);
        assert_eq!(Request::parse_request_line(delete).unwrap().0, Method::Delete);
        assert_eq!(Request::parse_request_line(options).unwrap().0, Method::Options);
        assert_eq!(Request::parse_request_line(connect).unwrap().0, Method::Connect);
        assert!(Request::parse_request_line(bad1).is_err());
        assert!(Request::parse_request_line(bad2).is_err());
        assert!(Request::parse_request_line(bad3).is_err());
    }
}

#[cfg(test)]
mod http_version {
    use crate::{Request, Version};

    #[test]
    fn from_str_request_line() {
        let zeronine = "GET /test HTTP/0.9";
        let onezero = "GET /test HTTP/1.0";
        let oneone = "GET /test HTTP/1.1";
        let twozero = "POST /test HTTP/2.0";
        assert_eq!(Request::parse_request_line(zeronine).unwrap().2, Version::ZeroDotNine);
        assert_eq!(Request::parse_request_line(onezero).unwrap().2, Version::OneDotZero);
        assert_eq!(Request::parse_request_line(oneone).unwrap().2, Version::OneDotOne);
        assert_eq!(Request::parse_request_line(twozero).unwrap().2, Version::TwoDotZero);
    }
}

#[cfg(test)]
mod http_status {
    use crate::Status;

    #[test]
    fn from_str_code() {
        let s100 = "100";
        let s201 = "201";
        let s301 = "301";
        let s403 = "403";
        let s500 = "500";
        let bad = "800";
        assert_eq!(s100.parse::<Status>().unwrap(), Status(100));
        assert_eq!(s201.parse::<Status>().unwrap(), Status(201));
        assert_eq!(s301.parse::<Status>().unwrap(), Status(301));
        assert_eq!(s403.parse::<Status>().unwrap(), Status(403));
        assert_eq!(s500.parse::<Status>().unwrap(), Status(500));
        assert!(bad.parse::<Status>().is_err());
    }
}

#[cfg(test)]
mod request {
    use std::collections::BTreeMap;
    use crate::consts::{ACCEPT, ACCEPT_ENCODING, CONNECTION, HOST, USER_AGENT};
    use crate::header::names::HdrRepr;
    use crate::{HeaderName, HeadersMap, Request};

    #[test]
    fn multiple_headers_from_str() {
        let test = "\
            Accept: */*\r\n\
            Accept-Encoding: gzip, deflate, br\r\n\
            Connection: keep-alive\r\n\
            Host: example.com\r\n\
            User-Agent: xh/0.19.3\r\n\
            Pineapple: pizza\r\n\r\n";

        let expected: HeadersMap = BTreeMap::from([
            (ACCEPT, "*/*".into()),
            (ACCEPT_ENCODING, "gzip, deflate, br".into()),
            (CONNECTION, "keep-alive".into()),
            (HOST, "example.com".into()),
            (USER_AGENT, "xh/0.19.3".into()),
            (
                HeaderName {
                    inner: HdrRepr::Custom(Vec::from("pineapple")),
                },
                "pizza".into(),
            ),
        ]);

        let mut output: HeadersMap = BTreeMap::new();

        for line in test.split('\n') {
            let trim = line.trim();

            if trim.is_empty() { break; }

            let (name, value) = Request::parse_header(trim).unwrap();
            output.insert(name, value);
        }

        assert_eq!(output.len(), expected.len());
        assert!(output.iter()
                      .zip(expected)
                      .all(|((k_out, v_out), (k_exp, v_exp))| {
                          *k_out == k_exp && *v_out == v_exp
                      }));
    }
}

#[cfg(test)]
mod utils {
    use crate::trim_whitespace_bytes;

    #[test]
    fn trim_wspace_bytes() {
        assert_eq!(trim_whitespace_bytes(b"  test"), b"test");
        assert_eq!(trim_whitespace_bytes(b"test    "), b"test");
        assert_eq!(trim_whitespace_bytes(b"         test       "), b"test");
        assert_eq!(
            trim_whitespace_bytes(b"  Hello \nworld       "),
            b"Hello \nworld"
        );
        assert_eq!(trim_whitespace_bytes(b"\t  \nx\t  x\r\x0c"), b"x\t  x");
        assert_eq!(trim_whitespace_bytes(b"                   "), b"");
        assert_eq!(trim_whitespace_bytes(b" "), b"");
        assert_eq!(trim_whitespace_bytes(b"x"), b"x");
        assert_eq!(trim_whitespace_bytes(b""), b"");
    }
}

#[cfg(test)]
mod client {
    use crate::{Client, HeaderValue as HdVal, Status, Version};
    use crate::consts::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS as ACAC,
        ACCESS_CONTROL_ALLOW_ORIGIN as ACAO, CONNECTION as CONN,
        CONTENT_LENGTH as CONLEN, CONTENT_TYPE as CONTYPE, DATE, LOCATION,
        SERVER
    };
    const SERVER_ADDR: &str = "httpbin.org:80";

    // Responds with the status code corresponding to `code`.
    macro_rules! test_by_status_code {
        ($label:ident: $code:literal) => {
            #[test]
            fn $label() {
                let uri = format!("/status/{}", $code);
                let mut client = Client::http()
                    .addr(&SERVER_ADDR)
                    .uri(&uri)
                    .build()
                    .unwrap();
                client.send().unwrap();
                let res = client.recv().unwrap();

                assert_eq!(res.version, Version::OneDotOne);
                assert_eq!(res.status, Status($code));
                assert_eq!(res.headers.get(&ACAC), Some(&HdVal::new(b"true")));
                assert_eq!(res.headers.get(&ACAO), Some(&HdVal::new(b"*")));
                if $code == 101 {
                    assert_eq!(res.headers.get(&CONN),
                        Some(&HdVal::new(b"upgrade")));
                } else  {
                    assert_eq!(res.headers.get(&CONN),
                        Some(&HdVal::new(b"keep-alive")));
                    assert_eq!(res.headers.get(&CONLEN),
                        Some(&HdVal::new(b"0")));
                }
                if $code == 303 {
                    assert_eq!(res.headers.get(&LOCATION),
                        Some(&HdVal::new(b"/redirect/1")));
                } else {
                    assert_eq!(res.headers.get(&CONTYPE),
                        Some(&HdVal::new(b"text/html; charset=utf-8")));
                }
                assert_eq!(res.headers.get(&SERVER),
                    Some(&HdVal::new(b"gunicorn/19.9.0")));
                assert!(res.headers.contains_key(&DATE));
                //assert_eq!(res.body, Vec::new());
            }
        };
    }

    test_by_status_code!(parse_1xx_response: 101);
    test_by_status_code!(parse_2xx_response: 202);
    test_by_status_code!(parse_3xx_response: 303);
    test_by_status_code!(parse_4xx_response: 404);
    test_by_status_code!(parse_5xx_response: 505);
}
