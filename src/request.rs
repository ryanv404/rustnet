use std::{
    borrow::Cow,
    fmt,
    io::{BufRead, BufReader},
    net::TcpStream,
};

use crate::{Header, HeaderName, Method, NetError, Version, util::trim_whitespace};

//GET / HTTP/1.1
//Accept: */* (*/ for syntax coloring bug)
//Accept-Encoding: gzip, deflate, br
//Connection: keep-alive
//Host: example.com
//User-Agent: xh/0.19.3

type NetResult<T> = Result<T, NetError>;
type RequestLine = (Method, Vec<u8>, Version);

pub struct Request {
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
        method: Method,
        uri: &[u8],
        version: Version,
        headers: &[Header],
        body: &[u8],
    ) -> Self {
        let uri = uri.to_owned();
        let body = body.to_owned();
        let headers = headers.to_owned();
        Self { method, uri, version, headers, body }
    }

    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.push(Header::new(name.as_bytes(), value.as_bytes()));
    }

    #[must_use]
    pub fn from_reader(reader: &mut BufReader<TcpStream>) -> NetResult<Self> {
        let mut headers = vec![];

        let mut lines = reader
            .split(b'\n')
            .map(|line| line.unwrap());

        let (method, uri, version) = lines
            .next()
            .as_ref()
            .ok_or(NetError::BadRequestLine)
            .and_then(|line| Self::parse_request_line(line))?;

        while let Some(line) = lines.next().as_ref() {
            let line = trim_whitespace(line);
            if line.is_empty() {
                break;
            }

            let header = Self::parse_header(line)?;
            headers.push(header);
        }

        //let body = Self::parse_body().unwrap_or(Vec::new());
        let body = Vec::new();

        Ok(Self { method, uri, version, headers, body })
    }

    fn parse_request_line(buf: &[u8]) -> NetResult<RequestLine> {
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

    fn parse_header(buf: &[u8]) -> NetResult<Header> {
        let line = trim_whitespace(buf);
        let mut tokens = line.splitn(2, |&b| b == b':');
        let (name, value) = (tokens.next(), tokens.next());

        name.and_then(|name| Some(Header::new(name, value?)))
            .ok_or(NetError::BadRequestHeader)
    }

//    fn parse_body(buf: &[u8]) -> Option<Vec<u8>> {
//        None
//    }

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::{
        CacheControlValue,
        ContentLengthValue,
        ContentTypeValue
    };

    #[test]
    fn test_request_headers_search() {
        let cache = Header::from(CacheControlValue::NoCache);
        let c_type = Header::from(ContentTypeValue::TextHtml);
        let c_len = Header::from(ContentLengthValue::from(100u64));
        let headers = vec![cache.clone(), c_len.clone(), c_type.clone()];
        let req = Request {
            method: Method::Get,
            uri: b"/about".to_vec(),
            version: Version::OneDotOne,
            headers,
            body: Vec::new(),
        };

        assert!(req.has_header(HeaderName::CacheControl));
        assert!(req.has_header(HeaderName::ContentLength));
        assert!(req.has_header(HeaderName::ContentType));
        assert!(!req.has_header(HeaderName::Host));
        assert_eq!(req.get_header(HeaderName::CacheControl), Some(&cache));
        assert_eq!(req.get_header(HeaderName::ContentLength), Some(&c_len));
        assert_eq!(req.get_header(HeaderName::ContentType), Some(&c_type));
        assert_eq!(req.get_header(HeaderName::Host), None);
        assert_ne!(req.get_header(HeaderName::ContentType), Some(&c_len));
    }
}
