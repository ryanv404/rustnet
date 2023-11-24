use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{
    BufRead, ErrorKind as IoErrorKind, Result as IoResult, Write,
};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::str;

use crate::consts::{
	ACCEPT, CONTENT_ENCODING, CONTENT_TYPE, DEFAULT_NAME, HOST, MAX_HEADERS,
    USER_AGENT,
};
use crate::{
    Connection, HeaderName, HeaderValue, HeadersMap, Method, NetError,
    NetResult, ParseErrorKind, Route, Version,
};

/// An HTTP request builder object.
#[derive(Clone, Debug)]
pub struct RequestBuilder<A: ToSocketAddrs> {
    pub addr: Option<A>,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub method: Option<Method>,
    pub path: Option<String>,
    pub version: Option<Version>,
    pub headers: Option<HeadersMap>,
    pub body: Option<Vec<u8>>,
}

impl<A: ToSocketAddrs> Default for RequestBuilder<A> {
    fn default() -> Self {
        Self {
            addr: None,
            ip: None,
            port: None,
            method: None,
            path: None,
            version: None,
            headers: None,
            body: None,
        }
    }
}

impl<A: ToSocketAddrs> RequestBuilder<A> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = Some(method);
        self
    }

    pub fn path(&mut self, path: &str) -> &mut Self {
        self.path = Some(path.to_string());
        self
    }

    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    /// Sets the remote host's IP address.
    pub fn ip(&mut self, ip: &str) -> &mut Self {
        self.ip = Some(ip.to_string());
        self
    }

    /// Sets the remote host's port.
    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = Some(port);
        self
    }

    /// Sets the socket address of the remote server.
    pub fn addr(&mut self, addr: A) -> &mut Self {
        self.addr = Some(addr);
        self
    }

    pub fn add_header(&mut self, name: &str, value: &str) -> &mut Self {
        let name = HeaderName::from(name);
        let value = HeaderValue::from(value);

        if let Some(map) = self.headers.as_mut() {
            map.entry(name)
                .and_modify(|val| *val = value.clone())
                .or_insert(value);
        } else {
            self.headers = Some(BTreeMap::from([(name, value)]));
        }

        self
    }

    pub fn body(&mut self, data: &[u8]) -> &mut Self {
        if !data.is_empty() {
            self.body = Some(data.to_vec());
        }

        self
    }

    pub fn build(&mut self) -> IoResult<Request> {
        let conn = {
			if let Some(addr) = self.addr.take() {
                let stream = TcpStream::connect(addr)?;
                Connection::try_from(stream).ok()
            } else if self.ip.is_some() && self.port.is_some() {
				let ip = self.ip.take().unwrap();
				let port = self.port.take().unwrap();
				let addr = format!("{ip}:{port}");
                let stream = TcpStream::connect(addr)?;
                Connection::try_from(stream).ok()
            } else {
                None
            }
        };

        let method = self.method.take().unwrap_or_default();
        let version = self.version.take().unwrap_or_default();
        let path = self.path.take().unwrap_or_else(|| String::from("/"));
		let headers = self.headers.take();
		let body = self.body.take();

		Ok(Request {
            conn,
            method,
            path,
            version,
            headers,
            body,
        })
    }

    pub fn send(&mut self) -> IoResult<Request> {
        let mut req = self.build()?;
        req.send()?;
        Ok(req)
    }
}

/// Represents the components of an HTTP request.
pub struct Request {
    pub conn: Option<Connection>,
    pub method: Method,
    pub path: String,
    pub version: Version,
    pub headers: Option<HeadersMap>,
    pub body: Option<Vec<u8>>,
}

impl Default for Request {
    fn default() -> Self {
		Self {
            conn: None,
            method: Method::default(),
            path: "/".to_string(),
            version: Version::default(),
            headers: None,
            body: None,
        }
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		// The request line.
		writeln!(f, "{}", self.request_line())?;

		// The request headers.
		if let Some(headers) = self.headers.as_ref() {
			for (name, value) in headers {
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
		f.debug_struct("Request")
            .field("remote", &"Connection { ... }")
			.field("method", &self.method)
			.field("path", &self.path)
			.field("version", &self.version)
			.field("headers", &self.headers)
            .field("body", &self.body.as_ref().map_or_else(
                || None,
                |body| if self.body_is_printable() {
                    Some(body)
                } else {
                    None
                })
            )
            .finish()
    }
}

impl TryFrom<Connection> for Request {
    type Error = NetError;

    /// Parse a `Request` from a `Connection`.
    fn try_from(mut conn: Connection) -> NetResult<Self> {
        let mut buf = String::new();

        // Parse the request line.
        let (method, path, version) = {
            match conn.read_line(&mut buf) {
                Err(e) => return Err(NetError::from(e)),
                Ok(0) => return Err(NetError::UnexpectedEof),
                Ok(_) => Self::parse_request_line(&buf)?,
            }
        };

        let mut num = 0;
        let mut headers = BTreeMap::new();

        // Parse the request headers.
        while num <= MAX_HEADERS {
            buf.clear();

            match conn.read_line(&mut buf) {
                Err(e) => return Err(NetError::from(e)),
                Ok(0) => return Err(NetError::UnexpectedEof),
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

        let conn = Some(conn);
        let headers = Some(headers);

        Ok(Self {
            conn,
            method,
            path,
            version,
            headers,
            body,
        })
    }
}

impl Request {
    /// Parses the first line of an HTTP request.
    ///
    /// request-line = method SP request-target SP HTTP-version
    pub fn parse_request_line(line: &str) -> NetResult<(Method, String, Version)> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Err(ParseErrorKind::ReqLine)?;
        }

        let mut tokens = trimmed.splitn(3, ' ').map(str::trim);

        let (Some(method), Some(path), Some(version)) = (
            tokens.next(), tokens.next(), tokens.next()
        ) else {
            return Err(ParseErrorKind::ReqLine)?;
        };

        Ok((method.parse()?, path.to_string(), version.parse()?))
    }

    /// Parses a line into a header field name and value.
    ///
    /// field-line = field-name ":" OWS field-value OWS
    pub fn parse_header(line: &str) -> NetResult<(HeaderName, HeaderValue)> {
        let mut tokens = line.splitn(2, ':').map(str::trim);

        let (Some(name), Some(value)) = (tokens.next(), tokens.next()) else {
            return Err(ParseErrorKind::Header)?;
        };

        Ok((HeaderName::from(name), HeaderValue::from(value)))
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
    pub const fn headers(&self) -> Option<&HeadersMap> {
        self.headers.as_ref()
    }

    /// Default set of request headers.
    pub fn default_headers(&mut self) {
        if let Some(remote) = self.remote_addr() {
            let host = format!("{}:{}", remote.ip(), remote.port());
            self.set_header(HOST, host.into());
        }

        self.set_header(ACCEPT, "*/*".into());
        self.set_header(USER_AGENT, DEFAULT_NAME.into());
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.as_ref().map_or(false, |headers| {
            headers.contains_key(name)
        })
    }

    /// Returns the header value for the given `HeaderName`, if present.
    #[must_use]
    pub fn header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.as_ref().map_or(None, 
            |headers| headers.get(name)
        )
    }

	/// Adds or modifies the header field represented by `HeaderName`.
    pub fn set_header(&mut self, name: HeaderName, val: HeaderValue) {
        if let Some(headers) = self.headers.as_mut() {
            headers.entry(name)
                .and_modify(|v| *v = val.clone())
                .or_insert(val);
        } else {
            self.headers = Some(BTreeMap::from([(name, val)]));
        }
    }

    /// Returns all of the request headers as a String.
    #[must_use]
    pub fn headers_to_string(&self) -> String {
        self.headers.as_ref().map_or(String::new(),
            |headers| headers.iter().fold(
                String::new(),
                |mut acc, (name, value)| {
                    let header = format!("{name}: {value}\n");
                    acc.push_str(&header);
                    acc
                })
        )
    }

    /// The `SocketAddr` of the remote connection.
    #[must_use]
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.conn
            .as_ref()
            .map(|conn| conn.remote_addr)
    }

    /// The `IpAddr` of the remote connection.
    #[must_use]
    pub fn remote_ip(&self) -> Option<IpAddr> {
        self.remote_addr()
            .as_ref()
            .map(|remote| remote.ip())
    }

    /// The port being used by the remote connection.
    #[must_use]
    pub fn remote_port(&self) -> Option<u16> {
        self.remote_addr()
            .as_ref()
            .map(|remote| remote.port())
    }

    /// The `SocketAddr` of the local connection.
    #[must_use]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.conn
            .as_ref()
            .map(|conn| conn.local_addr)
    }

    /// The `IpAddr` of the local connection.
    #[must_use]
    pub fn local_ip(&self) -> Option<IpAddr> {
        self.local_addr()
            .as_ref()
            .map(|local| local.ip())
    }

    /// The port being used by the local connection.
    #[must_use]
    pub fn local_port(&self) -> Option<u16> {
        self.local_addr()
            .as_ref()
            .map(|local| local.port())
    }

    /// Logs the response status and request line.
    pub fn log_status(&self, status_code: u16) {
        let remote = self
            .remote_addr()
            .as_ref()
            .map_or(
                String::from("?"),
                |remote| remote.to_string()
            );

        println!(
            "[{remote}|{status_code}] {} {}",
            self.method(),
            self.path()
        );
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

	/// Returns the request body as a copy-on-write string.
    #[must_use]
    pub fn body_to_string(&self) -> Cow<'_, str> {
        if let Some(body) = self.body.as_ref() {
            if !body.is_empty() && self.body_is_printable() {
                return String::from_utf8_lossy(body);
            }
        }

        String::new().into()
    }

	/// Write this `Request` as an HTTP request to a remote host.
    #[must_use]
    pub fn send(&mut self) -> NetResult<()> {
		let request_line = self.request_line();
        self.default_headers();

        let writer = self
            .conn
            .as_mut()
            .map(|conn| conn.writer.by_ref())
            .ok_or(NetError::WriteError(IoErrorKind::NotConnected))?;

        // The request line.
		writer.write_all(request_line.as_bytes())?;
		writer.write_all(b"\r\n")?;

		// The request headers.
		if let Some(headers) = self.headers.as_ref() {
            for (name, value) in headers {
                writer.write_all(format!("{name}: ").as_bytes())?;
                writer.write_all(value.as_bytes())?;
                writer.write_all(b"\r\n")?;
            }
		}

		// End of the headers section.
		writer.write_all(b"\r\n")?;

		// The request message body, if present.
		if let Some(body) = self.body.as_ref() {
			if !body.is_empty() {
				writer.write_all(body)?;
			}
		}

		writer.flush()?;
        Ok(())
    }
}
