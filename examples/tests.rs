use rustnet::{
    Method, Version, Request, Header, HeaderName, util::trim_whitespace,
};

fn main() {
    test_parse_request_line();
    test_parse_request_headers();
}

fn test_parse_request_line() {
    let test1 = b"GET /test HTTP/1.1";
    let test2 = b"POST /test HTTP/2.0";
    let test3 = b"   GET /test HTTP/1.1 Content-Type: text/plain  ";
    let test4 = b"foo bar baz";
    let test5 = b"GET /test";
    let test6 = b"GET";

    let expected1 = (Method::Get, "/test".as_bytes().to_vec(), Version::OneDotOne);
    let expected2 = (Method::Post, "/test".as_bytes().to_vec(), Version::TwoDotZero);

    eprintln!("OUTPUT1: {:?}", Request::parse_request_line(test1));
    eprintln!("EXPECTED_METHOD1: {:?}", expected1.0);
    eprintln!("EXPECTED_URI1: {:?}", String::from_utf8_lossy(&expected1.1));
    eprintln!("EXPECTED_VERSION1: {:?}\n", expected1.2);

    eprintln!("OUTPUT2: {:?}", Request::parse_request_line(test2));
    eprintln!("EXPECTED_METHOD2: {:?}", expected2.0);
    eprintln!("EXPECTED_URI2: {:?}", String::from_utf8_lossy(&expected2.1));
    eprintln!("EXPECTED_VERSION2: {:?}\n", expected2.2);

    eprintln!("OUTPUT3: {:?}", Request::parse_request_line(test3));
    eprintln!("EXPECTED_METHOD3: {:?}", expected1.0);
    eprintln!("EXPECTED_URI3: {:?}", String::from_utf8_lossy(&expected1.1));
    eprintln!("EXPECTED_VERSION3: {:?}\n", expected1.2);

    eprintln!("OUTPUT4: {:?}\n", Request::parse_request_line(test4));
    eprintln!("OUTPUT5: {:?}\n", Request::parse_request_line(test5));
    eprintln!("OUTPUT6: {:?}\n", Request::parse_request_line(test6));
}

fn test_parse_request_headers() {
    let test_headers = "\
        Accept: */*\r\n\
        Accept-Encoding: gzip, deflate, br\r\n\
        Connection: keep-alive\r\n\
        Host: example.com\r\n\
        User-Agent: xh/0.19.3\r\n\
        Pineapple: pizza\r\n\r\n"
        .as_bytes();

    let expected = vec![
        Header::new(HeaderName::Accept.as_str().as_bytes(), b"*/*"),
        Header::new(HeaderName::AcceptEncoding.as_str().as_bytes(), b"gzip, deflate, br"),
        Header::new(HeaderName::Connection.as_str().as_bytes(), b"keep-alive"),
        Header::new(HeaderName::Host.as_str().as_bytes(), b"example.com"),
        Header::new(HeaderName::UserAgent.as_str().as_bytes(), b"xh/0.19.3"),
        Header::new(HeaderName::Unknown(String::from("Pineapple")).as_str().as_bytes(), b"pizza")
    ];

    let mut output = vec![];

    for line in test_headers.split(|&b| b == b'\n') {
        let line = trim_whitespace(line);
        if line.is_empty() {
            break;
        }

        let header = Request::parse_header(line).unwrap();
        output.push(header);
    }

    assert_eq!(&output[..], &expected[..]);
}
