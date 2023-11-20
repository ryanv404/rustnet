use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufRead, Read, Write};
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use crate::consts::{
    CONTENT_LENGTH, CONTENT_TYPE, MAX_HEADERS,
};
use crate::{
    HeaderName, HeaderValue, HeadersMap, Method, NetReader,
    NetWriter, Request, Response, Status, Version,
};

/// Builder for the `Client` object.
#[derive(Clone, Debug)]
pub struct ClientBuilder<A: ToSocketAddrs> {
    pub method: Option<Method>,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub addr: Option<A>,
    pub path: Option<String>,
    pub version: Option<Version>,
    pub headers: Option<HeadersMap>,
    pub body: Option<Vec<u8>>,
}

impl<A: ToSocketAddrs> Default for ClientBuilder<A> {
    fn default() -> Self {
        Self {
            method: None,
            ip: None,
            port: None,
            addr: None,
            path: None,
            version: None,
            headers: None,
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
        self.method = Some(method);
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
        self.version = Some(version);
        self
    }

    /// Adds a header field line to the request.
    pub fn header(&mut self, name: HeaderName, value: &str) -> &mut Self {
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

	/// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.as_ref().map_or(false, |h| h.contains_key(name))
    }

	/// Updates the Content-Length and Content-Type headers based on
	/// the client's body field.
    pub fn update_content_headers(&mut self) {
		if let Some(body) = self.body.as_ref() {
			if body.is_empty() {
				// Body is Some but is empty.
				self.body = None;
				
				if let Some(headers) = self.headers.as_mut() {
					// Body is empty and headers are present.
					headers.remove(&CONTENT_LENGTH);
					headers.remove(&CONTENT_TYPE);
				}
			} else if let Some(headers) = self.headers.as_mut() {
				// Body is not empty and headers are present.
				let len = body.len();
                headers.entry(CONTENT_LENGTH).or_insert_with(|| len.into());
                headers.entry(CONTENT_TYPE).or_insert_with(|| "text/plain".into());
			}
		} else if let Some(headers) = self.headers.as_mut() {
			// Body is None and headers are present.
			if headers.contains_key(&CONTENT_LENGTH) {
				headers.remove(&CONTENT_LENGTH);
			}
			
			if headers.contains_key(&CONTENT_TYPE) {
				headers.remove(&CONTENT_TYPE);
			}
		}
	}

	/// Sets the request body.
	pub fn body(&mut self, data: &[u8]) -> &mut Self {
		if !data.is_empty() {
			self.body = Some(data.to_vec());
		}
		
		self
	}
	
    /// Builds and returns a new `Client` instance.
    pub fn build(&mut self) -> IoResult<Client> {
        let stream = {
			if let Some(addr) = self.addr.take() {
				TcpStream::connect(addr)?
            } else if self.ip.is_some() && self.port.is_some() {
				let ip = self.ip.take().unwrap();
				let port = self.port.take().unwrap();
				let addr = format!("{ip}:{port}");

				TcpStream::connect(addr)?
            } else {
                return Err(IoError::from(IoErrorKind::InvalidInput));
            }
        };

        let local_addr = stream.local_addr()?;
		let remote_addr = stream.peer_addr()?;

        let reader = NetReader::from(stream.try_clone()?);
        let writer = NetWriter::from(stream);

        let method = self.method.take().unwrap_or_default();
        let version = self.version.take().unwrap_or_default();
        let path = self.path.take().unwrap_or_else(|| String::from("/"));

		self.update_content_headers();

		let headers = self
            .headers
            .take()
            .unwrap_or_else(|| Request::default_headers(&remote_addr));


		let body = self.body.take();

		let req = Request {
            remote_addr,
            method,
            path,
            version,
            headers,
            body,
        };

        Ok(Client {
            req,
            local_addr,
            reader,
            writer,
        })
    }

    pub fn send(&mut self) -> IoResult<Client> {
        let mut client = self.build()?;
        client.send()?;
        Ok(client)
    }
}

/// An HTTP client that can send and receive messages with a remote host.
#[derive(Debug)]
pub struct Client {
	pub req: Request,
	pub local_addr: SocketAddr,
    pub reader: NetReader,
    pub writer: NetWriter,
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		self.req.fmt(f)
	}
}

impl Write for Client {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.writer.write_all(buf)
    }
}

impl Read for Client {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.reader.read(buf)
    }
}

impl BufRead for Client {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt);
    }
}

impl Client {
    /// Sends a GET request to the provided URL, returning a `Response`.
   pub fn get(_url: &str) -> IoResult<Response> {
		todo!();
	}

    /// Returns a new `ClientBuilder` instance.
    #[must_use]
    pub fn http<A: ToSocketAddrs>() -> ClientBuilder<A> {
        ClientBuilder::new()
    }

    /// Returns the method.
    pub const fn method(&self) -> Method {
        self.req.method
    }

	/// Returns the URI path to the target resource.
    pub fn path(&self) -> &str {
        &self.req.path
    }

    /// Returns the protocol version.
    pub const fn version(&self) -> Version {
        self.req.version
    }

    /// Returns a reference to the request headers map.
    pub const fn headers(&self) -> &HeadersMap {
        self.req.headers()
    }

	/// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.req.has_header(name)
    }

    /// Returns the header value for the given `HeaderName`, if present.
    #[must_use]
    pub fn header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.req.header(name)
    }

	/// Adds or modifies the header field represented by `HeaderName`.
    pub fn set_header(&mut self, name: HeaderName, val: HeaderValue) {
        self.req.set_header(name, val);
    }

    /// Returns a formatted string of all of the request headers.
    pub fn headers_to_string(&self) -> String {
        if self.req.headers.is_empty() {
            String::new()
        } else {
            self.req
				.headers
                .iter()
                .fold(String::new(), |mut acc, (name, value)| {
                    let header = format!("{name}: {value}\r\n");
                    acc.push_str(&header);
                    acc
                })
        }
    }

    /// Returns a reference to the request body, if present.
    pub const fn body(&self) -> Option<&Vec<u8>> {
        self.req.body()
    }

    /// Returns the local socket address.
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns the remote server's socket address.
    pub const fn remote_addr(&self) -> SocketAddr {
        self.req.remote_addr
    }

    /// Returns the request line as a String.
    pub fn request_line(&self) -> String {
		self.req.request_line()
    }

    /// Sends an HTTP request to the remote host.
    pub fn send(&mut self) -> IoResult<()> {
		let writer = self.writer.by_ref();

		// The request line.
		let request_line = self.req.request_line();
		writer.write_all(request_line.as_bytes())?;
		writer.write_all(b"\r\n")?;

		// The request headers.
		for (name, value) in &self.req.headers {
			writer.write_all(format!("{name}: ").as_bytes())?;
			writer.write_all(value.as_bytes())?;
			writer.write_all(b"\r\n")?;
		}

		// End of the headers section.
		writer.write_all(b"\r\n")?;

		// The request message body, if present.
		if let Some(body) = self.req.body.as_ref() {
			if !body.is_empty() {
				writer.write_all(body)?;
			}
		}

		writer.flush()?;
        Ok(())
	}

    /// Receives an HTTP response from the remote host.
    pub fn recv(&mut self) -> IoResult<Response> {
        let (version, status) = self.parse_status_line()?;
        let headers = self.parse_headers()?;

		let body = {
			// Only parse the body if a valid Content-Length is present.
            headers.get(&CONTENT_LENGTH).and_then(|val| {
				let s_len = val.to_string();
                s_len.parse::<usize>().map_or(None, |len| self.parse_body(len))
			})
		};

        let path = self.req.path.clone();
        let method = self.req.method;

		Ok(Response {
            method,
            path,
            version,
            status,
            headers,
            body,
        })
    }

    /// Parses the first line of a response into a `Version` and `Status`.
    pub fn parse_status_line(&mut self) -> IoResult<(Version, Status)> {
        let mut buf = String::new();

        match self.read_line(&mut buf) {
            Err(e) => Err(e),
            Ok(0) => Err(IoError::from(IoErrorKind::UnexpectedEof)),
            Ok(_) => {
                let line = buf.trim();

                if line.is_empty() {
                    let payload = "response status line is empty".to_string();
                    return Err(IoError::new(IoErrorKind::Other, payload));
                }

                let mut tok = line.splitn(3, ' ').map(str::trim);

                let tokens = (tok.next(), tok.next(), tok.next());

                let (Some(ver), Some(code), Some(msg)) = tokens else {
                    let payload = "cannot parse the response status line".to_string();
                    return Err(IoError::new(IoErrorKind::Other, payload));
                };

                let Ok(version) = ver.parse::<Version>() else {
                    let payload = format!("cannot parse the HTTP version: {ver}");
                    return Err(IoError::new(IoErrorKind::Other, payload));
                };

                if code.eq_ignore_ascii_case("200") {
                    Ok((version, Status(200)))
                } else if let Ok(status) = code.parse::<Status>() {
                    Ok((version, status))
                } else {
                    let payload = format!("cannot parse status code: {code} ({msg})");
                    Err(IoError::new(IoErrorKind::Other, payload))
                }
            }
        }
    }

    // Reads and parses the headers section into a BTreeMap.
    pub fn parse_headers(&mut self) -> IoResult<HeadersMap> {
        let mut num = 0;
        let mut line = String::new();
        let mut headers: HeadersMap = BTreeMap::new();

        while num <= MAX_HEADERS {
            line.clear();

            match self.read_line(&mut line) {
                Err(e) => return Err(e),
                Ok(0) => return Err(IoError::from(IoErrorKind::UnexpectedEof)),
                Ok(_) => {
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        return Ok(headers);
                    }

                    let (name, value) = Request::parse_header(trimmed)?;
                    headers.insert(name, value);
                    num += 1;
                }
            }
        }

        Err(IoError::new(IoErrorKind::Other, String::from("too many headers")))
    }

    pub fn parse_body(&mut self, len: usize) -> Option<Vec<u8>> {
        if len == 0 {
			return None;
		}

		let Ok(num_bytes) = u64::try_from(len) else {
			return None;
		};

		let mut body = Vec::with_capacity(len);
		let mut handle = self.reader.by_ref().take(num_bytes);
	
		// TODO: handle chunked data and partial reads.
		if handle.read_to_end(&mut body).is_ok() {
            Some(body)
		} else {
		    None
		}
	}
}
