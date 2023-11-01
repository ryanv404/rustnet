use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufRead, ErrorKind::UnexpectedEof};
use std::net::{IpAddr, SocketAddr};
use std::str;

use crate::consts::{CACHE_CONTROL, CONTENT_LENGTH};
use crate::{
    HeaderName, HeadersMap, HeaderValue, Method, NetError, NetResult,
    RemoteConnect, Route, Version
};

//GET / HTTP/1.1
//Accept: */* (*/ for syntax coloring bug)
//Accept-Encoding: gzip, deflate, br
//Connection: keep-alive
//Host: example.com
//User-Agent: xh/0.19.3

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub remote_addr: SocketAddr,
    pub method: Method,
    pub uri: String,
    pub version: Version,
    pub headers: HeadersMap,
    pub body: Vec<u8>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            remote_addr: SocketAddr::new([127, 0, 0, 1].into(), 8787),
            method: Method::default(),
            uri: "/".to_string(),
            version: Version::default(),
            headers: Self::default_headers(),
            body: Vec::new(),
        }
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {} {}", &self.version, &self.method, &self.uri)
    }
}

impl Request {
    #[must_use]
    pub fn new(
        remote_addr: SocketAddr,
        method: Method,
        uri: String,
        version: Version,
        headers: HeadersMap,
        body: Vec<u8>,
    ) -> Self {
        Self {
            remote_addr,
            method,
            uri,
            version,
            headers,
            body,
        }
    }

    /// Tries to parse the first line of a request.
    pub fn parse_request_line(line: &str) -> NetResult<(Method, String, Version)> {
        let trim = line.trim();

        dbg!(trim);

        if trim.is_empty() {
            return Err(NetError::ParseError("request line"));
        }

        let mut tok = trim.splitn(3, ' ').map(str::trim);
        let tokens = (tok.next(), tok.next(), tok.next());

        dbg!(&tokens);

        if let (Some(m), Some(u), Some(v)) = tokens {
            Ok((m.parse()?, u.to_string(), v.parse()?))
        } else {
            Err(NetError::ParseError("request line"))
        }
    }

    /// Tries to parse a string slice into a `HeaderName` and `HeaderValue`.
    pub fn parse_header(input: &str) -> NetResult<(HeaderName, HeaderValue)> {
        let mut tok = input.splitn(2, ':').map(str::trim);
        let tokens = (tok.next(), tok.next());

        if let (Some(name), Some(value)) = tokens {
            Ok((name.parse()?, value.into()))
        } else {
            Err(NetError::ParseError("request header"))
        }
    }

    /// Parse a `Request` sent by a `RemoteConnect`.
    pub fn from_client(client: &mut RemoteConnect) -> NetResult<Self> {
        let remote_addr = client.remote_addr;

        let mut buf = String::new();

        // Parse the request line.
        let (method, uri, version) = {
            match client.read_line(&mut buf) {
                Err(e) => {
                    return Err(NetError::from(e));
                }
                Ok(0) => {
                    return Err(NetError::from_kind(UnexpectedEof));
                }
                Ok(_) => Self::parse_request_line(&buf)?,
            }
        };

        let mut headers = BTreeMap::new();

        // Parse the request headers.
        loop {
            buf.clear();

            match client.read_line(&mut buf) {
                Err(e) => {
                    return Err(NetError::from(e));
                }
                Ok(0) => {
                    return Err(NetError::from_kind(UnexpectedEof));
                }
                Ok(_) => {
                    let trim = buf.trim();

                    if trim.is_empty() {
                        break;
                    }

                    let (name, value) = Self::parse_header(trim)?;
                    headers.insert(name, value);
                }
            }
        }

        // Parse the request body.
        let body = Self::parse_body(b"");

        Ok(Self {
            remote_addr,
            method,
            uri,
            version,
            headers,
            body,
        })
    }

    #[must_use]
    pub const fn parse_body(_buf: &[u8]) -> Vec<u8> {
        Vec::new()
    }

    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.method
    }

    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    #[must_use]
    pub fn route(&self) -> Route {
        Route::new(self.method, &self.uri)
    }

    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    pub fn headers(&self) -> &HeadersMap {
        &self.headers
    }

    /// Default set of request headers.
    #[must_use]
    pub fn default_headers() -> HeadersMap {
        BTreeMap::from([
            (CACHE_CONTROL, "no-cache".into()),
            (CONTENT_LENGTH, "0".into()),
        ])
    }

    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains_key(name)
    }

    #[must_use]
    pub fn get_header_value(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    #[must_use]
    pub const fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    #[must_use]
    pub const fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
    }

    #[must_use]
    pub const fn remote_port(&self) -> u16 {
        self.remote_addr.port()
    }

    /// Logs the response status and request line for new requests.
    pub fn log_with_status(&self, status: u16) {
        println!("[{}|{status}] {}", self.remote_ip(), self);
    }

    /// Logs the request headers, request line, and response status for new
    /// requests.
    pub fn log_verbose(&self, status: u16) {
        println!("[{}|{status}] {}", self.remote_ip(), self);

        for (name, val) in &self.headers {
            println!("{name}: {}", String::from_utf8_lossy(val.as_bytes()));
        }
    }
}
