use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{
    BufRead, Error as IoError, ErrorKind as IoErrorKind, Read, Write,
};
use std::io::Result as IoResult;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};

use crate::consts::{
    ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, HOST, USER_AGENT,
};
use crate::{
    Connection, HeaderName, HeaderValue, Headers, Method, NetResult, Request,
    NetError, NetReader, RequestLine, Response, Version,
    ParseErrorKind,
};

/// An HTTP request builder object.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientBuilder<A: ToSocketAddrs> {
    pub method: Method,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub addr: Option<A>,
    pub path: Option<String>,
    pub version: Version,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

impl<A: ToSocketAddrs> Default for ClientBuilder<A> {
    fn default() -> Self {
        Self {
            method: Method::Get,
            ip: None,
            port: None,
            addr: None,
            path: None,
            version: Version::OneDotOne,
            headers: Headers::new(),
            body: None,
        }
    }
}

impl<A: ToSocketAddrs> ClientBuilder<A> {
	/// Returns a new `ClientBuilder` instance.
	#[must_use]
    pub fn new() -> Self {
        Self::default()
    }

	/// Sets the HTTP method.
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = method;
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

	/// Sets the URI path to the target resource.
    pub fn path(&mut self, path: &str) -> &mut Self {
        self.path = Some(path.to_string());
        self
    }

	/// Sets the protocol version.
	pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = version;
        self
    }

    /// Sets a request header field line.
    pub fn set_header(&mut self, name: &str, value: &str) -> &mut Self {
        self.headers.insert(name.into(), value.into());
        self
    }

	/// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

	/// Sets the request body and adds Content-Type and Content-Length
    /// headers.
	pub fn body(&mut self, data: &[u8]) -> &mut Self {
		if data.is_empty() {
            return self;
        }

        self.headers.insert(CONTENT_LENGTH, data.len().into());
        self.headers.insert(CONTENT_TYPE, "text/plain".into());

        self.body = Some(data.to_vec());
        self
	}

    /// Builds and returns a new `Client` instance.
    pub fn build(&mut self) -> NetResult<Client> {
        let conn = {
			if let Some(addr) = self.addr.as_ref() {
				let stream = TcpStream::connect(addr)?;
                Connection::try_from(stream)?
            } else if self.ip.is_some() && self.port.is_some() {
				let addr = format!(
                    "{}:{}",
                    self.ip.as_ref().unwrap(),
                    self.port.as_ref().unwrap()
                );

                let stream = TcpStream::connect(addr)?;
                Connection::try_from(stream)?
            } else {
                return Err(IoError::from(IoErrorKind::InvalidInput).into());
            }
        };

        let path = self.path.as_ref().map_or_else(
            || String::from("/"),
            |path| path.to_owned()
        );

        let request_line = RequestLine::new(self.method, path, self.version);
        let headers = self.headers.clone();
        let body = self.body.clone();
        let req = Request { request_line, headers, body };

        Ok(Client { conn, req })
    }

    /// Sends an HTTP request and then returns a `Client` instance.
    pub fn send(&mut self) -> IoResult<Client> {
        let mut client = self.build()?;
        client.send()?;
        Ok(client)
    }
}

/// An HTTP client that can send and receive messages with a remote host.
#[derive(Debug)]
pub struct Client {
    pub conn: Connection,
	pub req: Request,
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		self.req.fmt(f)
	}
}

impl Write for Client {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.conn.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.conn.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.conn.write_all(buf)
    }
}

impl Read for Client {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.conn.read(buf)
    }
}

impl BufRead for Client {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.conn.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.conn.consume(amt);
    }
}

impl Client {
    /// Sends a GET request to the provided URI, returning the `Client` and
	/// the `Response`.
    pub fn get(uri: &str) -> NetResult<(Self, Response, String)> {
		let (addr, path) = Self::parse_uri(uri)?;
        let mut client = ClientBuilder::new()
            .addr(&addr)
            .path(&path)
            .build()?;

        client.send()?;
        let res = client.recv()?;

        Ok((client, res, addr))
	}

    /// Returns a new `ClientBuilder` instance.
    #[must_use]
    pub fn new<A: ToSocketAddrs>() -> ClientBuilder<A> {
        ClientBuilder::new()
    }
    

    /// Returns the method.
    pub const fn method(&self) -> &Method {
        self.req.method()
    }

	/// Returns the URI path to the target resource.
    pub fn path(&self) -> &str {
        self.req.path()
    }

    /// Returns the protocol version.
    pub const fn version(&self) -> &Version {
        self.req.version()
    }

    /// Returns a reference to the request headers.
    pub const fn headers(&self) -> &Headers {
        self.req.headers()
    }

	/// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.req.has_header(name)
    }

    /// Returns the header value for the given `HeaderName`, if present.
    #[must_use]
    pub fn get_header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.req.get_header(name)
    }

	/// Adds or modifies the header field represented by `HeaderName`.
    pub fn set_header(&mut self, name: HeaderName, val: HeaderValue) {
        self.req.set_header(name, val);
    }

    /// Returns a formatted string of all of the request headers.
    pub fn headers_to_string(&self) -> String {
        self.req.headers_to_string()
    }

    /// Adds default header values for Accept, Host, and User-Agent, not
    /// already set.
    pub fn include_default_headers(&mut self) {
        if !self.req.headers.contains(&HOST) {
            self.req.headers.insert_host(self.remote_ip(), self.remote_port());
        }

        if !self.req.headers.contains(&USER_AGENT) {
            self.req.headers.insert_user_agent();
        }

        if !self.req.headers.contains(&ACCEPT) {
            self.req.headers.insert_accept_all();
        }
    }

    /// Returns a reference to the request body, if present.
    pub const fn body(&self) -> Option<&Vec<u8>> {
        self.req.body()
    }

        /// Returns the `SocketAddr` of the remote connection.
        #[must_use]
        pub fn remote_addr(&self) -> &SocketAddr {
            &self.conn.remote_addr
        }

        /// Returns the `IpAddr` of the remote connection.
        #[must_use]
        pub fn remote_ip(&self) -> IpAddr {
            self.remote_addr().ip()
        }

        /// Returns the port being used by the remote connection.
        #[must_use]
        pub fn remote_port(&self) -> u16 {
            self.remote_addr().port()
        }

        /// Returns the `SocketAddr` of the local connection.
        #[must_use]
        pub fn local_addr(&self) -> &SocketAddr {
            &self.conn.local_addr
        }

        /// Returns the `IpAddr` of the local connection.
        #[must_use]
        pub fn local_ip(&self) -> IpAddr {
            self.local_addr().ip()
        }

        /// Returns the port being used by the local connection.
        #[must_use]
        pub fn local_port(&self) -> u16 {
            self.local_addr().port()
        }

    /// Returns the request line as a String.
    pub fn request_line(&self) -> String {
		self.req.request_line()
    }

    /// Writes an HTTP request to a remote host.
    #[must_use]
    pub fn send(&mut self) -> NetResult<()> {
        self.include_default_headers();

        // The request line.
		self.conn.writer.write_all(self.request_line().as_bytes())?;
		self.conn.writer.write_all(b"\r\n")?;

		// The request headers.
        for (name, value) in &self.req.headers.0 {
            let header = format!("{name}: {value}\r\n");
            self.conn.writer.write_all(header.as_bytes())?;
		}

		// End of the headers section.
		self.conn.writer.write_all(b"\r\n")?;

		// The request message body, if present.
		if let Some(body) = self.req.body.as_ref() {
			if !body.is_empty() {
				self.conn.writer.write_all(body)?;
			}
		}

		self.conn.writer.flush()?;
        Ok(())
    }

    /// Reads and parses the message body based on the Content-Type and
    /// Content-Length headers values.
    #[must_use]
    pub fn parse_body(
        reader: &mut NetReader,
        len_val: usize,
        type_val: &str
    ) -> Option<Vec<u8>> {
        if len_val == 0 {
            return None;
        }

        let Ok(num_bytes) = u64::try_from(len_val) else {
            return None;
        };

        if !type_val.contains("text") && !type_val.contains("application") {
            return None;
        }

        let mut body = Vec::with_capacity(len_val);
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
                return Err(ParseErrorKind::Uri.into());
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

    /// Receives an HTTP response from the remote host.
    pub fn recv(&mut self) -> NetResult<Response> {
        Response::recv(self.conn.try_clone()?, *self.req.method())
    }
}
