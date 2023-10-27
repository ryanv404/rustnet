use std::{
    io::BufRead,
    fmt::{Display, Formatter, Result as FmtResult},
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use crate::{
    Header, HeaderName, Method, NetError, NetResult, RemoteClient, Route, Version,
};

//GET / HTTP/1.1
//Accept: */* (*/ for syntax coloring bug)
//Accept-Encoding: gzip, deflate, br
//Connection: keep-alive
//Host: example.com
//User-Agent: xh/0.19.3

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestLine {
    pub method: Method,
    pub uri: String,
    pub version: Version,
}

impl Default for RequestLine {
    fn default() -> Self {
        Self {
            method: Method::Get,
            uri: "/".to_string(),
            version: Version::OneDotOne
        }
    }
}

impl Display for RequestLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {} {}", &self.method, &self.uri, &self.version)
    }
}

impl FromStr for RequestLine {
    type Err = NetError;

    fn from_str(input: &str) -> NetResult<Self> {
        let trimmed_buf = input.trim();
        if trimmed_buf.is_empty() {
            return Err(NetError::ParseError("request line"));
        }

        let tokens: Vec<&str> = trimmed_buf.splitn(3, ' ').collect();
        if tokens.len() != 3 {
            return Err(NetError::ParseError("request line"));
        }

        let method = Method::from_str(tokens[0])?;
        let uri = tokens[1].trim().to_string();
        let version = Version::from_str(tokens[2])?;

        if uri.is_empty() {
            Err(NetError::ParseError("uri"))
        } else {
            Ok(Self::new(method, &uri, version))
        }
    }
}

impl RequestLine {
    #[must_use]
    pub fn new(method: Method, uri: &str, version: Version) -> Self {
        Self { method, uri: uri.to_string(), version }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub remote_addr: SocketAddr,
    pub request_line: RequestLine,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

impl Request {
    #[must_use]
    pub fn new(
        remote_addr: SocketAddr,
        request_line: RequestLine,
        headers: Vec<Header>,
        body: Vec<u8>
    ) -> Self {
        Self {
            remote_addr,
            request_line,
            headers,
            body
        }
    }

    pub fn from_client(client: &mut RemoteClient) -> NetResult<Self> {
        let mut headers = vec![];
        let mut line_buf = String::new();

        let remote_addr = client.remote_addr.clone();

        // Parse the request line.
        let request_line = {
            match client.read_line(&mut line_buf) {
                Ok(0) => return Err(NetError::EarlyEof),
                Err(e) => return Err(NetError::from(e)),
                Ok(_) => RequestLine::from_str(&line_buf)?,
            }
        };

        // Parse the request headers.
        loop {
            line_buf.clear();

            match client.read_line(&mut line_buf) {
                Ok(0) => return Err(NetError::EarlyEof),
                Err(e) => return Err(NetError::from(e)),
                Ok(_) => {
                    let trimmed_buf = line_buf.trim();
                    if trimmed_buf.is_empty() {
                        break;
                    }

                    headers.push(Header::from_str(trimmed_buf)?);
                },
            }
        }

        // Parse the request body.
        let body = Self::parse_body(b"");

        Ok(Self {
            remote_addr,
            request_line,
            headers,
            body
        })
    }

    #[must_use]
    pub const fn parse_body(_buf: &[u8]) -> Vec<u8> {
        Vec::new()
    }

    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.request_line.method
    }

    #[must_use]
    pub fn uri(&self) -> &str {
        &self.request_line.uri
    }

    #[must_use]
    pub fn route(&self) -> Route {
        Route::new(self.request_line.method, &self.request_line.uri)
    }

    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.request_line.version
    }

    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.iter().any(|h| h.name == *name)
    }

    #[must_use]
    pub fn get_header(&self, name: &HeaderName) -> Option<&Header> {
        self.headers.iter().find(|&h| h.name == *name)
    }

    #[must_use]
    pub const fn request_line(&self) -> &RequestLine {
        &self.request_line
    }

    #[must_use]
    pub const fn remote_addr(&self) -> &SocketAddr {
        &self.remote_addr
    }

    #[must_use]
    pub fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
    }

    #[must_use]
    pub fn remote_port(&self) -> u16 {
        self.remote_addr.port()
    }

    pub fn log_connection_status(&self, status: u16) {
        let request_line = self.request_line();
        println!("[{}|{status}] {request_line}", self.remote_ip());
    }
}
