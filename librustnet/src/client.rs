use std::borrow::ToOwned;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufRead, ErrorKind as IoErrorKind, Read, Write};
use std::io::Result as IoResult;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};

use crate::consts::{
    ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, HOST, USER_AGENT,
};
use crate::{
    Connection, HeaderName, HeaderValue, Headers, Method, NetError, NetResult,
    ParseErrorKind, RequestLine, Response, Request, Version,
};

/// An HTTP request builder object.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientBuilder<A>
where
    A: ToSocketAddrs
{
    pub method: Method,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub addr: Option<A>,
    pub path: Option<String>,
    pub version: Version,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

impl<A> Default for ClientBuilder<A>
where
    A: ToSocketAddrs
{
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

impl<A> ClientBuilder<A>
where
    A: ToSocketAddrs
{
	/// Returns a new `ClientBuilder` instance.
	#[must_use]
    pub fn new() -> Self {
        Self::default()
    }

	/// Sets the HTTP method.
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

	/// Sets the remote host's IP address.
    pub fn ip(mut self, ip: &str) -> Self {
        self.ip = Some(ip.to_string());
        self
    }

	/// Sets the remote host's port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

	/// Sets the socket address of the remote server.
    pub fn addr(mut self, addr: A) -> Self {
        self.addr = Some(addr);
        self
    }

	/// Sets the URI path to the target resource.
    pub fn path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

	/// Sets the protocol version.
	pub fn version(mut self, version: Version) -> Self {
        self.version = version;
        self
    }

    /// Sets a request header field line.
    pub fn insert_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

	/// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

	/// Sets the request body and adds Content-Type and Content-Length
    /// headers.
	pub fn body(mut self, data: &[u8]) -> Self {
		if data.is_empty() {
            return self;
        }

        self.headers.insert(CONTENT_LENGTH, data.len().into());
        self.headers.insert(CONTENT_TYPE, "text/plain".as_bytes().into());
        self.body = Some(data.to_vec());
        self
	}

    /// Builds and returns a new `Client` instance.
    pub fn build(self) -> NetResult<Client> {
        let conn = {
			if let Some(addr) = self.addr.as_ref() {
				let stream = TcpStream::connect(addr)?;
                Connection::try_from(stream)?
            } else if self.ip.is_some() && self.port.is_some() {
                let ip = self.ip.as_ref().unwrap();
                let port = self.port.as_ref().unwrap();

                let addr = format!("{ip}:{port}");
                let stream = TcpStream::connect(addr)?;

                Connection::try_from(stream)?
            } else {
                return Err(IoErrorKind::InvalidInput.into());
            }
        };

        let path = self.path.as_ref()
            .map_or_else(
                || String::from("/"),
                |s| ToOwned::to_owned(s)
            );

        let request_line = RequestLine::new(self.method, path, self.version);
        let headers = self.headers.clone();
        let body = self.body.clone();

        let req = Request { request_line, headers, body, conn };

        Ok(Client { req })
    }

    /// Sends an HTTP request and then returns a `Client` instance.
    pub fn send(self) -> IoResult<Client> {
        let mut client = self.build()?;
        client.send()?;
        Ok(client)
    }
}

/// An HTTP client that can send and receive messages with a remote host.
#[derive(Debug)]
pub struct Client {
	pub req: Request,
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		self.req.fmt(f)
	}
}

impl Write for Client {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.req.conn.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.req.conn.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.req.conn.write_all(buf)
    }
}

impl Read for Client {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.req.conn.read(buf)
    }
}

impl BufRead for Client {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.req.conn.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.req.conn.consume(amt);
    }
}

impl Client {
    /// Sends a GET request to the provided URI.
    pub fn get(uri: &str) -> NetResult<(Self, Response)> {
		let (addr, path) = Self::parse_uri(uri)?;

        let mut client = Client::builder()
            .addr(&addr)
            .path(&path)
            .send()?;

        let res = client.recv()?;

        Ok((client, res))
	}

    /// Returns a new `ClientBuilder` instance.
    #[must_use]
    pub fn builder<A>() -> ClientBuilder<A>
    where
        A: ToSocketAddrs
    {
        ClientBuilder::new()
    }
    

    /// Returns the method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.req.method()
    }

	/// Returns the URI path to the target resource.
    #[must_use]
    pub fn path(&self) -> &str {
        self.req.path()
    }

    /// Returns the protocol version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.req.version()
    }

    /// Returns a reference to the request headers.
    #[must_use]
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
    pub fn insert_header(&mut self, name: HeaderName, val: HeaderValue) {
        self.req.insert_header(name, val);
    }

    /// Returns a formatted string of all of the request headers.
    #[must_use]
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
    #[must_use]
    pub const fn body(&self) -> Option<&Vec<u8>> {
        self.req.body()
    }

    /// Returns the `SocketAddr` of the remote connection.
    #[must_use]
    pub const fn remote_addr(&self) -> SocketAddr {
        self.req.conn.remote_addr
    }

    /// Returns the `IpAddr` of the remote connection.
    #[must_use]
    pub const fn remote_ip(&self) -> IpAddr {
        self.remote_addr().ip()
    }

    /// Returns the port being used by the remote connection.
    #[must_use]
    pub const fn remote_port(&self) -> u16 {
        self.remote_addr().port()
    }

    /// Returns the `SocketAddr` of the local connection.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.req.conn.local_addr
    }

    /// Returns the `IpAddr` of the local connection.
    #[must_use]
    pub const fn local_ip(&self) -> IpAddr {
        self.local_addr().ip()
    }

    /// Returns the port being used by the local connection.
    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.local_addr().port()
    }

    /// Returns the request line as a String.
    #[must_use]
    pub fn request_line(&self) -> String {
		self.req.request_line()
    }

    /// Writes an HTTP request to a remote host.
    pub fn send(&mut self) -> NetResult<()> {
        self.include_default_headers();

        // The request line.
		self.req.conn.write_all(self.request_line().as_bytes())?;
		self.req.conn.write_all(b"\r\n")?;

		// The request headers.
        for (name, value) in &self.req.headers.0 {
            let header = format!("{name}: {value}\r\n");
            self.req.conn.write_all(header.as_bytes())?;
		}

		// End of the headers section.
		self.req.conn.write_all(b"\r\n")?;

		// The request message body, if present.
		if let Some(body) = self.req.body.as_ref() {
			if !body.is_empty() {
				self.req.conn.write_all(body)?;
			}
		}

		self.req.conn.flush()?;
        Ok(())
    }

    /// Parses a string slice into a host address and a URI path.
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
        Response::recv(self.req.conn.try_clone()?, self.req.method())
    }
}
