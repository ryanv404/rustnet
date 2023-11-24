use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufRead, ErrorKind as IoErrorKind, Read, Result as IoResult, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::str;

use crate::consts::{
	ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, DEFAULT_NAME, HOST, MAX_HEADERS,
	USER_AGENT,
};
use crate::{
    Connection, Header, HeaderName, HeaderValue, HeadersSet, Method, NetError,
    NetReader, NetResult, ParseErrorKind, Route, Version,
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
    pub headers: HeadersSet,
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
            headers: BTreeSet<Header>::new(),
            body: None,
        }
    }
}

impl<A: ToSocketAddrs> RequestBuilder<A> {
    /// Returns a `RequestBuilder instance`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP method.
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = Some(method);
        self
    }

    /// Sets the URI path.
    pub fn path(&mut self, path: &str) -> &mut Self {
        self.path = Some(path.to_string());
        self
    }

    /// Sets the protocol version.
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

    /// Adds a new header or updates the header value if it is already present.
    pub fn add_header(&mut self, name: &str, value: &str) -> &mut Self {
        let name = HeaderName::from(name);
        let value = HeaderValue::from(value);
        let header = Header::new(name, value);

        if self.headers.is_empty() {
            self.headers = BTreeSet<Header>::from([header]);
        } else {
            self.headers.insert(header);
        }

        self
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Sets the content of the request body.
    pub fn body(&mut self, data: &[u8]) -> &mut Self {
        if !data.is_empty() {
            self.body = Some(data.to_vec());
        }

        self
    }

    /// Returns a `Request` instance from the `RequestBuilder`.
    pub fn build(&mut self) -> NetResult<Request> {
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
        let path = self.path.take().unwrap_or_else(|| String::from("/"));
        let version = self.version.take().unwrap_or_default();
        let request_line = RequestLine::new(method, path, version);

		let headers = self.headers.unwrap_or_else(|| BTreeSet::<Header>::new());
		let body = self.body.take();

		Ok(Request {
            conn,
            request_line,
            headers,
            body,
        })
    }

    /// Sends an HTTP request and then returns the `Request` instance.
    pub fn send(&mut self) -> NetResult<Request> {
        let mut req = self.build()?;
        req.send()?;
        Ok(req)
    }
}

/// Represents the first line of an HTTP request.
#[derive(Debug)]
pub struct RequestLine {
    pub method: Method,
    pub path: String,
    pub version: Version,
}

impl Default for RequestLine {
    fn default() -> Self {
        Self {
            method: Method::Get,
            path: String::from("/"),
            version: Version::OneDotOne
        }
    }
}
impl Display for RequestLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {} {}", &self.method, &self.path, &self.version)
    }
}

impl RequestLine {
    /// Returns a new `RequestLine` instance.
    pub fn new(method: Method, path: String, version: Version) -> Self {
        Self { method, path, version }
    }

    /// Returns the HTTP method.
    #[must_use]
    pub fn method(&self) -> Method {
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

    /// Parses a string slice into a URI path, a `Method`, and a `Version`
    /// returning a `RequestLine` object.
    pub fn parse(line: &str) -> NetResult<Self> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Err(ParseErrorKind::RequestLine)?;
        }

        let mut tokens = trimmed.splitn(3, ' ').map(str::trim);

        let (Some(method), Some(path), Some(version)) = (
            tokens.next(), tokens.next(), tokens.next()
        ) else {
            return Err(ParseErrorKind::RequestLine)?;
        };

        let method = method.parse()?;
        let path = path.to_string();
        let version = version.parse()?;

        Ok(Self { method, path, version })
    }
}

/// Represents the components of an HTTP request.
pub struct Request {
    pub conn: Option<Connection>,
    pub request_line: RequestLine,
    pub headers: HeadersSet,
    pub body: Option<Vec<u8>>,
}

impl Default for Request {
    fn default() -> Self {
		Self {
            conn: None,
            request_line: RequestLine::default(),
            headers: BTreeSet::<Header>::new(),
            body: None,
        }
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		// The request line.
		writeln!(f, "{}", self.request_line)?;

		// The request headers.
		for header in self.headers.iter() {
			writeln!(f, "{header}")?;
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
			.field("request_line", &self.request_line)
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

impl Request {
    /// Returns the HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.request_line.method
    }

    /// Returns the URI path to the target resource.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.request_line.path
    }

    /// Returns the `Route` representation of the target resource.
    #[must_use]
    pub fn route(&self) -> Route {
        self.request_line.route()
    }

    /// Returns the HTTP version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.request_line.version
    }

    /// Returns the request line as a String.
    #[must_use]
    pub fn request_line(&self) -> String {
        self.request_line.to_string()
    }

    /// Returns a reference to the `Request` object's headers.
    #[must_use]
    pub const fn headers(&self) -> &HeadersSet {
        self.headers.as_ref()
    }

    /// Default set of request headers.
    pub fn default_headers(&mut self) {
        if let Some(remote) = self.remote_addr() {
            let host = format!("{}:{}", remote.ip(), remote.port());
            self.headers.insert(Header::new(HOST, host.into()));
        }

        self.set_header(Header::new(ACCEPT, "*/*".into()));
        self.set_header(Header::new(USER_AGENT, DEFAULT_NAME.into()));
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.as_ref().map_or(false, |headers| {
            headers.contains_key(name)
        })
    }

    /// Returns the header value for the given `Header`, if present.
    #[must_use]
    pub fn header(&self, name: &Header) -> Option<&Header> {
        self.headers.get(name)
    }

	/// Adds or modifies the header field represented by `HeaderName`.
    pub fn set_header(&mut self, name: HeaderName, val: HeaderValue) {
        let header = Header::new(name, val);

        if self.headers.is_empty() {
            self.headers = BTreeSet<Header>::from([header]);
        } else {
            self.headers.insert(Header::new(header));
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

	/// Returns true if the body has a text/* or application/* Content-Type header.
    #[must_use]
    pub fn body_is_printable(&self) -> bool {
        if let Some(header) = self.headers.get(&CONTENT_TYPE) {
            let body_type = header.value.to_string();
            body_type.contains("text") || body_type.contains("application")
        } else {
            false
        }
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
        self.headers.iter().for_each(|header| {
            writer.write_all(format!("{header}\r\n").as_bytes());
		});

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

    /// Reads an HTTP request from a `Connection` and parses it into a `Request` object.
    pub fn recv(mut conn: Connection) -> NetResult<Self> {
        let mut header_num = 0;
        let mut line = String::new();
        let mut headers = BTreeSet<Header>::new();

        // Parse the request line.
        let request_line = match conn.read_line(&mut line) {
            Err(e) => return Err(NetError::from(e)),
            Ok(0) => return Err(NetError::UnexpectedEof),
            Ok(_) => RequestLine::parse(&line)?,
        };

        // Parse the request headers.
        while header_num <= MAX_HEADERS {
            line.clear();

            match conn.read_line(&mut line) {
                Err(e) => return Err(NetError::from(e)),
                Ok(0) => return Err(NetError::UnexpectedEof),
                Ok(_) => {
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        break;
                    }

                    let header = Header::parse(trimmed)?;
                    headers.insert(header);
                    header_num += 1;
                }
            }
        }

        // Parse the request body.
        let body_len = headers
            .get(&CONTENT_LENGTH)
            .and_then(|cont_len| {
                let len_str = cont_len.to_string();
                len_str.parse::<usize>().ok()
            });

        let body_type = headers
            .get(&CONTENT_TYPE)
            .and_then(|cont_type| Some(cont_type.to_string()));

        let body = Self::parse_body(&mut conn.reader, body_len, body_type.as_ref());

        let conn = Some(conn);
        let headers = Some(headers);

        Ok(Self { conn, request_line, headers, body })
    }

    /// Reads and parses the message body based on the Content-Type and
    /// Content-Length headers values.
    #[must_use]
    pub fn parse_body(
        reader: &mut NetReader,
        cont_len: Option<usize>,
        cont_type: Option<&String>
    ) -> Option<Vec<u8>> {
        let (Some(body_len), Some(body_type)) = (cont_len, cont_type) else {
            return None;
        };

        if body_len == 0 {
    		return None;
    	}

        let Ok(num_bytes) = u64::try_from(body_len) else {
            return None;
        };

        if !body_type.contains("text") && !body_type.contains("application") {
    		return None;
    	}

        let mut body = Vec::with_capacity(body_len);
        let mut rdr = reader.take(num_bytes);

        // TODO: handle chunked data and partial reads.
        if rdr.read_to_end(&mut body).is_ok() {
            if !body.is_empty() {
                return Some(body);
            }
        }

        None
    }

    /// Parses a string slice into a host address and a URI path.
    #[must_use]
    pub fn parse_uri(uri: &str) -> NetResult<(String, String)> {
    	let uri = uri.trim();

    	if let Some((scheme, rest)) = uri.split_once("://") {
            // If "://" is present, we expect a URI like "http://httpbin.org".
            if scheme.is_empty() || rest.is_empty() {
    			return Err(ParseErrorKind::Uri)?;
    		}

    		match scheme {
                "http" => match rest.split_once('/') {
                    // Next "/" after the scheme, if present, starts the
                    // path segment.
    				Some((addr, path)) if path.is_empty() && addr.contains(':') => {
                        // Example: http://httpbin.org:80/
                        Ok((addr.to_string(), String::from("/")))
                    },
                    Some((addr, path)) if path.is_empty() => {
                        // Example: http://httpbin.org/
                        Ok((format!("{addr}:80"), String::from("/")))
    				},
    				Some((addr, path)) if addr.contains(':') => {
                        // Example: http://httpbin.org:80/json
                        Ok((addr.to_string(), format!("/{path}")))
    				},
                    Some((addr, path)) => {
                        // Example: http://httpbin.org/json
                        Ok((format!("{addr}:80"), format!("/{path}")))
                    },
    				None if rest.contains(':') => {
                        // Example: http://httpbin.org:80
                        Ok((rest.to_string(), String::from("/")))
    				},
                    None => {
                        // Example: http://httpbin.org
                        Ok((format!("{rest}:80"), String::from("/")))
    				},
    			},
                "https" => Err(NetError::HttpsNotImplemented),
                _ => Err(ParseErrorKind::Uri)?,
    		}
    	} else if let Some((addr, path)) = uri.split_once('/') {
    		if addr.is_empty() {
    			return Err(ParseErrorKind::Uri)?;
    		}

    		let addr = if addr.contains(':') {
    			addr.to_string()
    		} else {
    			format!("{addr}:80")
    		};

    		let path = if path.is_empty() {
    			String::from("/")
    		} else {
    			format!("/{path}")
    		};

    		Ok((addr, path))
    	} else if uri.contains(':') {
    		Ok((uri.to_string(), String::from("/")))
    	} else {
    		Ok((format!("{uri}:80"), String::from("/")))
    	}
    }
}
