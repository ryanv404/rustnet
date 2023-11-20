use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufRead, ErrorKind::UnexpectedEof};
use std::net::{IpAddr, SocketAddr};
use std::str;

use crate::consts::{
	ACCEPT, CONTENT_ENCODING, CONTENT_TYPE, HOST, MAX_HEADERS, USER_AGENT,
};
use crate::{
    HeaderName, HeaderValue, HeadersMap, Method, NetError, NetResult,
    RemoteConnect, Route, Version,
};

/// Represents the components of an HTTP request.
#[derive(Clone, Eq, PartialEq)]
pub struct Request {
    pub remote_addr: SocketAddr,
    pub method: Method,
    pub path: String,
    pub version: Version,
    pub headers: HeadersMap,
    pub body: Option<Vec<u8>>,
}

impl Default for Request {
    fn default() -> Self {
		let remote_addr = SocketAddr::new([127, 0, 0, 1].into(), 8787);

		Self {
            remote_addr,
            method: Method::default(),
            path: "/".to_string(),
            version: Version::default(),
            headers: Self::default_headers(&remote_addr),
            body: None,
        }
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		// The request line.
		writeln!(f, "{}", self.request_line())?;

		// The request headers.
		if !self.headers.is_empty() {
			for (name, value) in &self.headers {
				writeln!(f, "{name}: {value}")?;
			}
		}

		// The request message body, if present.
		if let Some(body) = self.body.as_ref() {
			if !body.is_empty() && self.body_is_printable() {
				let body = String::from_utf8_lossy(body);
				write!(f, "\n{body}")?;
			}
		}

		Ok(())
    }
}

impl Debug for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut dbg = f.debug_struct("Request");

		let dbg = dbg.field("remote_addr", &self.remote_addr)
			.field("method", &self.method)
			.field("path", &self.path)
			.field("version", &self.version)
			.field("headers", &self.headers);

		if self.body.is_none() || !self.body_is_printable() {
			dbg.field("body", &self.body).finish()
		} else {
			let body = self.body.as_ref().map(|b| String::from_utf8_lossy(b));
			dbg.field("body", &body).finish()
		}
	}
}

impl Request {
    /// Parses the first line of an HTTP request.
    ///
    /// request-line = method SP request-target SP HTTP-version
    pub fn parse_request_line(line: &str) -> NetResult<(Method, String, Version)> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Err(NetError::ParseError("request line"));
        }

        let mut tok = trimmed.splitn(3, ' ').map(str::trim);

        let tokens = (tok.next(), tok.next(), tok.next());

        if let (Some(meth), Some(path), Some(ver)) = tokens {
            Ok((meth.parse()?, path.to_string(), ver.parse()?))
        } else {
            Err(NetError::ParseError("request line"))
        }
    }

    /// Parses a line into a header field name and value.
    ///
    /// field-line = field-name ":" OWS field-value OWS
    pub fn parse_header(line: &str) -> NetResult<(HeaderName, HeaderValue)> {
        let mut tok = line.splitn(2, ':').map(str::trim);

        let tokens = (tok.next(), tok.next());

        if let (Some(name), Some(value)) = tokens {
            Ok((name.parse()?, value.into()))
        } else {
            Err(NetError::ParseError("header"))
        }
    }

    /// Parse a `Request` from a `RemoteConnect`.
    pub fn from_remote(remote: &mut RemoteConnect) -> NetResult<Self> {
        let remote_addr = remote.remote_addr;

        let mut buf = String::new();

        // Parse the request line.
        let (method, path, version) = {
            match remote.read_line(&mut buf) {
                Err(e) => return Err(NetError::from(e)),
                Ok(0) => return Err(NetError::from_kind(UnexpectedEof)),
                Ok(_) => Self::parse_request_line(&buf)?,
            }
        };

        let mut num = 0;
        let mut headers = BTreeMap::new();

        // Parse the request headers.
        while num <= MAX_HEADERS {
            buf.clear();

            match remote.read_line(&mut buf) {
                Err(e) => return Err(NetError::from(e)),
                Ok(0) => return Err(NetError::from_kind(UnexpectedEof)),
                Ok(_) => {
                    let trimmed = buf.trim();

                    if trimmed.is_empty() {
                        break;
                    }

                    let (name, value) = Self::parse_header(trimmed)?;
                    headers.insert(name, value);
                    num += 1;
                }
            }
        }

        // Parse the request body.
        let body = Self::parse_body(b"");

        Ok(Self {
            remote_addr,
            method,
            path,
            version,
            headers,
            body,
        })
    }

    /// Parses the request body.
    ///
    /// Presence of a request body depends on the Content-Length and
    /// Transfer-Encoding headers.
    #[must_use]
    pub const fn parse_body(_buf: &[u8]) -> Option<Vec<u8>> {
		// TODO
		None
    }

    /// Returns the HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the URI path to the target resource.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the `Route` representation of the target resource.
    #[must_use]
    pub fn route(&self) -> Route {
        Route::new(self.method, &self.path)
    }

    /// Returns the HTTP version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Returns the request line as a String.
    #[must_use]
    pub fn request_line(&self) -> String {
        format!("{} {} {}", &self.method, &self.path, &self.version)
    }

    /// Returns a reference to the `Request` object's headers.
    #[must_use]
    pub const fn headers(&self) -> &HeadersMap {
        &self.headers
    }

    /// Default set of request headers.
    #[must_use]
    pub fn default_headers(host: &SocketAddr) -> HeadersMap {
        let uagent = format!(
            "{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")
        );

		let host = format!("{}:{}", host.ip(), host.port());

		BTreeMap::from([
			(ACCEPT, "*/*".into()),
			(HOST, host.as_str().into()),
            (USER_AGENT, uagent.as_str().into()),
        ])
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains_key(name)
    }

    /// Returns the header value for the given `HeaderName`, if present.
    #[must_use]
    pub fn header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

	/// Adds or modifies the header field represented by `HeaderName`.
    pub fn set_header(&mut self, name: HeaderName, val: HeaderValue) {
        if self.has_header(&name) {
            self.headers.entry(name).and_modify(|v| *v = val);
        } else {
            self.headers.insert(name, val);
        }
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
    pub fn log_status(&self, status_code: u16) {
        println!("[{}|{status_code}] {}", self.remote_ip(), self.request_line());
    }

	/// Returns true if the body is unencoded and has a text or application
	/// Content-Type header.
    #[must_use]
    pub fn body_is_printable(&self) -> bool {
        if self.has_header(&CONTENT_ENCODING)
            || !self.has_header(&CONTENT_TYPE)
        {
            return false;
        }

        self.header(&CONTENT_TYPE).map_or(false, |ct| {
            let ct = ct.to_string();
            ct.contains("text") || ct.contains("application")
        })
    }

	/// Returns a referense to the request body, if present.
	#[must_use]
	pub const fn body(&self) -> Option<&Vec<u8>> {
		self.body.as_ref()
	}
}
