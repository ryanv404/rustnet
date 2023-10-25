use std::{
    borrow::Cow,
    fmt,
    io::{self, BufRead, BufReader},
    net::{IpAddr, SocketAddr, TcpStream},
};

use crate::{
    ArcRouter, Header, HeaderName, Method, NetError, Response, Version,
    util::trim_whitespace
};

//GET / HTTP/1.1
//Accept: */* (*/ for syntax coloring bug)
//Accept-Encoding: gzip, deflate, br
//Connection: keep-alive
//Host: example.com
//User-Agent: xh/0.19.3

type NetResult<T> = Result<T, NetError>;

type RequestLine = (Method, Vec<u8>, Version);

pub struct Request {
    pub remote_addr: Option<SocketAddr>,
    pub method: Method,
    pub uri: Vec<u8>,
    pub version: Version,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let body = if self.body.is_empty() { "No content." } else { "..." };

        f.debug_struct("Request")
            .field("method", &self.method)
            .field("uri", &String::from_utf8_lossy(&self.uri))
            .field("version", &self.version)
            .field("headers", &self.headers)
            .field("body", &body)
            .finish()
    }
}

impl Request {
    #[must_use]
    pub fn new(
        remote_addr: Option<SocketAddr>,
        method: Method,
        uri: &[u8],
        version: Version,
        headers: &[Header],
        body: &[u8],
    ) -> Self {
        let uri = uri.to_owned();
        let body = body.to_owned();
        let headers = headers.to_owned();

        Self { remote_addr, method, uri, version, headers, body }
    }

    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.push(Header::new(name.as_bytes(), value.as_bytes()));
    }

    #[must_use]
    pub fn from_reader(reader: &mut BufReader<TcpStream>) -> NetResult<Self> {
        let remote_addr = reader.get_ref().peer_addr().ok();

        let mut lines = reader
            .split(b'\n')
            .map(|line| line.unwrap());

        let (method, uri, version) = lines
            .next()
            .as_ref()
            .ok_or(NetError::BadRequestLine)
            .and_then(|line| Self::parse_request_line(line))?;

        let mut headers = vec![];

        while let Some(line) = lines.next().as_ref() {
            let line = trim_whitespace(line);
            if line.is_empty() {
                break;
            }

            let header = Self::parse_header(line)?;
            headers.push(header);
        }

        let body = Self::parse_body(b"");

        Ok(Self { remote_addr, method, uri, version, headers, body })
    }

    #[must_use]
    pub fn parse_request_line(buf: &[u8]) -> NetResult<RequestLine> {
        let line = trim_whitespace(buf);

        if line.is_empty() {
            return Err(NetError::BadRequestLine);
        }

        let mut tokens = line.split(|&b| b == b' ');

        let method = tokens.next().and_then(|t| Method::try_from(t).ok());
        let uri = tokens.next().map(|t| t.to_owned());
        let version = tokens.next().and_then(|t| Version::try_from(t).ok());

        version
            .and_then(|version| Some((method?, uri?, version)))
            .ok_or(NetError::BadRequestLine)
    }

    #[must_use]
    pub fn parse_header(buf: &[u8]) -> NetResult<Header> {
        let line = trim_whitespace(buf);
        let mut tokens = line.splitn(2, |&b| b == b':');
        let (name, value) = (tokens.next(), tokens.next());

        name.and_then(|name| Some(Header::new(name, value?)))
            .ok_or(NetError::BadRequestHeader)
    }

    #[must_use]
    pub fn parse_body(_buf: &[u8]) -> Vec<u8> {
        Vec::new()
    }

    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.method
    }

    #[must_use]
    pub fn uri(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.uri)
    }

    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    #[must_use]
    pub fn has_header(&self, name: HeaderName) -> bool {
        self.headers.iter().any(|h| h.name == name)
    }

    #[must_use]
    pub fn get_header(&self, name: HeaderName) -> Option<&Header> {
        self.headers.iter().find(|&h| h.name == name)
    }

    #[must_use]
    pub fn request_line(&self) -> String {
        format!("{} {} {}", self.method(), self.uri(), self.version())
    }

    #[must_use]
    pub fn remote_addr(&self) -> Option<&SocketAddr> {
        self.remote_addr.as_ref()
    }

    #[must_use]
    pub fn remote_ip(&self) -> Option<IpAddr> {
        self.remote_addr().map(|sock| sock.ip())
    }

    #[must_use]
    pub fn remote_port(&self) -> Option<u16> {
        self.remote_addr().map(|sock| sock.port())
    }

    pub fn log_connection_status(&self, status: u16) {
        if let Some(remote_ip) = self.remote_ip() {
            println!("[{remote_ip}|{status}] {}", self.request_line());
        } else {
            println!("[?|{status}] {}", self.request_line());
        }
    }

    #[must_use]
    pub fn respond(&self, router: &ArcRouter) -> io::Result<Response> {
        Response::from_request(self, router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::{
        CacheControlValue, ContentLengthValue, ContentTypeValue
    };

    #[test]
    fn test_request_headers_search() {
        let remote_addr = None;
        let cache = Header::from(CacheControlValue::NoCache);
        let c_type = Header::from(ContentTypeValue::TextHtml);
        let c_len = Header::from(ContentLengthValue::from(100u64));
        let headers = vec![cache.clone(), c_len.clone(), c_type.clone()];
        let req = Request {
            remote_addr,
            method: Method::Get,
            uri: b"/about".to_vec(),
            version: Version::OneDotOne,
            headers,
            body: Vec::new(),
        };

        assert_eq!(req.get_header(HeaderName::CacheControl), Some(&cache));
        assert_eq!(req.get_header(HeaderName::ContentLength), Some(&c_len));
        assert_eq!(req.get_header(HeaderName::ContentType), Some(&c_type));
        assert!(!req.has_header(HeaderName::Host));
        assert_ne!(req.get_header(HeaderName::ContentType), Some(&c_len));
    }

    #[test]
    fn test_parse_request_line() {
        let test1 = b"GET /test HTTP/1.1";
        let test2 = b"POST /test HTTP/2.0";
        let test3 = b"   GET /test HTTP/1.1 Content-Type: text/plain  ";
        let test4 = b"foo bar baz";
        let test5 = b"GET /test";
        let test6 = b"GET";
        let expected1 = (Method::Get, "/test".to_string().into_bytes(), Version::OneDotOne);
        let expected2 = (Method::Post, "/test".to_string().into_bytes(), Version::TwoDotZero);

        assert_eq!(Request::parse_request_line(test1).unwrap(), expected1);
        assert_eq!(Request::parse_request_line(test2).unwrap(), expected2);
        assert_eq!(Request::parse_request_line(test3).unwrap(), expected1);
        assert!(Request::parse_request_line(test4).is_err());
        assert!(Request::parse_request_line(test5).is_err());
        assert!(Request::parse_request_line(test6).is_err());
    }

    #[test]
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
}
