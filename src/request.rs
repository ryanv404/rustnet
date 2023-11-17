use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufRead, ErrorKind::UnexpectedEof};
use std::net::{IpAddr, SocketAddr};
use std::str;

use crate::consts::{ACCEPT, MAX_HEADERS, USER_AGENT};
use crate::{
    HeaderName, HeaderValue, HeadersMap, Method, NetError, NetResult,
    RemoteConnect, Route, Version,
};

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
    /// Parses the first line of an HTTP request into an HTTP method, URI, and HTTP version.
    pub fn parse_request_line(line: &str) -> NetResult<(Method, String, Version)> {
        let trim = line.trim();

        if trim.is_empty() {
            return Err(NetError::ParseError("request line"));
        }

        let mut tok = trim.splitn(3, ' ').map(str::trim);

        let tokens = (tok.next(), tok.next(), tok.next());

        if let (Some(meth), Some(uri), Some(ver)) = tokens {
            Ok((meth.parse()?, uri.to_string(), ver.parse()?))
        } else {
            Err(NetError::ParseError("request line"))
        }
    }

    /// Parses a string slice into a `HeaderName` and `HeaderValue`.
    pub fn parse_header(input: &str) -> NetResult<(HeaderName, HeaderValue)> {
        let mut tok = input.splitn(2, ':').map(str::trim);

        let tokens = (tok.next(), tok.next());

        if let (Some(name), Some(value)) = tokens {
            Ok((name.parse()?, value.into()))
        } else {
            Err(NetError::ParseError("request header"))
        }
    }

    /// Parse a `Request` from a `RemoteConnect`.
    pub fn from_remote(remote: &mut RemoteConnect) -> NetResult<Self> {
        let remote_addr = remote.remote_addr;

        let mut buf = String::new();

        // Parse the request line.
        let (method, uri, version) = {
            match remote.read_line(&mut buf) {
                Err(e) => return Err(NetError::from(e)),
                Ok(0) => return Err(NetError::from_kind(UnexpectedEof)),
                Ok(_) => Self::parse_request_line(&buf)?,
            }
        };

        let mut hdr_num = 0;
        let mut headers = BTreeMap::new();

        // Parse the request headers.
        //
        // Guard against DDoS by setting an upper limit to the number of
        // headers permitted.
        while hdr_num <= MAX_HEADERS {
            buf.clear();

            match remote.read_line(&mut buf) {
                Err(e) => return Err(NetError::from(e)),
                Ok(0) => return Err(NetError::from_kind(UnexpectedEof)),
                Ok(_) => {
                    let trim = buf.trim();

                    if trim.is_empty() {
                        break;
                    }

                    let (name, value) = Self::parse_header(trim)?;
                    headers.insert(name, value);

                    hdr_num += 1;
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

    /// Returns the HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the requested URI.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Returns the `Route` representation of the `Request`.
    #[must_use]
    pub fn route(&self) -> Route {
        Route::new(self.method, &self.uri)
    }

    /// Returns the HTTP version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Returns a reference to the `Request` object's headers.
    #[must_use]
    pub const fn headers(&self) -> &HeadersMap {
        &self.headers
    }

    /// Default set of request headers.
    #[must_use]
    pub fn default_headers() -> HeadersMap {
        let ua = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

        BTreeMap::from([
            (ACCEPT, "*/*".into()),
            (USER_AGENT, ua.as_str().into()),
        ])
    }

    /// Returns true if the `Request` contains the given `HeaderName`.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains_key(name)
    }

    /// Returns the `HeaderValue` associated with a given `HeaderName` key, if present.
    #[must_use]
    pub fn get_header_value(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// The `SocketAddr` of the remote connection.
    #[must_use]
    pub const fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// The `IpAddr` of the remote connection.
    #[must_use]
    pub const fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
    }

    /// The port being used by the remote connection.
    #[must_use]
    pub const fn remote_port(&self) -> u16 {
        self.remote_addr.port()
    }

    /// Logs the response status and request line.
    pub fn log(&self, status_code: u16) {
        println!("[{}|{status_code}] {}", self.remote_ip(), self);
    }
}
