use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufRead, ErrorKind as IoErrorKind, Read, Result as IoResult, Write};
use std::net::{IpAddr, SocketAddr};
use std::string::ToString;

use crate::consts::{
    CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, MAX_HEADERS,
};
use crate::{
    Connection, Header, HeaderName, HeaderValue, Headers, Method, NetError,
    NetReader, NetResult, Status, Version,
};

/// An HTTP response builder object.
#[derive(Debug)]
pub struct ResponseBuilder {
    pub conn: Connection,
    pub method: Option<Method>,
    pub version: Version,
    pub status: Option<Status>,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}

impl ResponseBuilder {
    /// Returns a `ResponseBuilder instance`.
    #[must_use]
    pub fn new(conn: Connection) -> Self {
        Self {
            conn,
            method: None,
            version: Version::OneDotOne,
            status: None,
            headers: Headers::new(),
            body: None
        }
    }

    /// Sets the protocol version.
    pub fn version(mut self, version: Version) -> Self {
        self.version = version;
        self
    }

    /// Sets the HTTP response status.
    pub fn status(mut self, status: Status) -> Self {
        self.status = Some(status);
        self
    }

    /// Sets the HTTP request method.
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Adds a new header or updates the header value if it is already present.
    pub fn insert_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Sets the content of the response body.
    pub fn body(mut self, data: &[u8]) -> Self {
        self.headers.insert(CONTENT_LENGTH, data.len().into());
        self.headers.insert(CONTENT_TYPE, Vec::from("text/plain").into());
        self.body = Some(data.to_vec());
        self
    }

    /// Returns a `Response` instance from the `ResponseBuilder`.
    pub fn build(mut self) -> NetResult<Response> {
        let conn = self.conn.try_clone()?;

        let method = self.method.take().unwrap_or_default();
        let status = self.status.take().unwrap_or_default();
        let status_line = StatusLine::new(self.version, status);

        let headers = self.headers;
		let body = self.body.take();

		Ok(Response {
            method,
            status_line,
            headers,
            body,
            conn,
        })
    }

    /// Sends an HTTP response and then returns the `Response` instance.
    pub fn send(self) -> NetResult<Response> {
        let mut res = self.build()?;
        res.send()?;
        Ok(res)
    }
}

/// Represents the status line of an HTTP response.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusLine {
    pub version: Version,
    pub status: Status,
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            version: Version::OneDotOne,
            status: Status(200)
        }
    }
}

impl Display for StatusLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {}", self.version, self.status)
    }
}

impl StatusLine {
    /// Returns a new `StatusLine` instance.
    #[must_use]
    pub const fn new(version: Version, status: Status) -> Self {
        Self { version, status }
    }

    /// Returns the protocol version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Returns the response status.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Returns the status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status.code()
    }

    /// Returns the status reason phrase.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status.msg()
    }

    /// Parses a string slice into a `StatusLine` object.
    pub fn parse(line: &str) -> NetResult<Self> {
        let mut tokens = line.trim_start().splitn(3, ' ');

        let version = Version::parse(tokens.next())?;
        let status = Status::parse(tokens.next())?;

        Ok(Self::new(version, status))
    }
}

/// Represents the components of an HTTP response.
pub struct Response {
    pub method: Method,
    pub status_line: StatusLine,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
    pub conn: Connection,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // The response status line.
        writeln!(f, "{}", self.status_line)?;

        // The response headers.
        for (name, value) in &self.headers.0 {
            writeln!(f, "{name}: {value}")?;
        }

        // The response body.
		if let Some(body) = self.body.as_ref() {
			if !body.is_empty() && self.body_is_printable() {
				let body = String::from_utf8_lossy(body);
				write!(f, "\n{body}")?;
			}
		}

		Ok(())
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("Response")
            .field("conn", &self.conn)
			.field("method", &self.method)
			.field("status_line", &self.status_line)
			.field("headers", &self.headers)
            .field("body", &self.body)
			.finish()
	}
}

impl Response {
    /// Returns the protocol version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.status_line.version
    }

    /// Returns the response's `Status` value.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status_line.status
    }

    /// Returns the status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_line.status.code()
    }

    /// Returns the status reason phrase.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status_line.status.msg()
    }

    /// Returns the `SocketAddr` of the remote half of the connection.
    #[must_use]
    pub const fn remote_addr(&self) -> SocketAddr {
        self.conn.remote_addr
    }

    /// Returns the `IpAddr` of the remote half of the connection.
    #[must_use]
    pub const fn remote_ip(&self) -> IpAddr {
        self.conn.remote_addr.ip()
    }

    /// Returns the port in use by the remote half of the connection.
    #[must_use]
    pub const fn remote_port(&self) -> u16 {
        self.conn.remote_addr.port()
    }

    /// Returns the `SocketAddr` of the local half of the connection.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.conn.local_addr
    }

    /// Returns the `IpAddr` of the local half of the  connection.
    #[must_use]
    pub const fn local_ip(&self) -> IpAddr {
        self.conn.local_addr.ip()
    }

    /// Returns the port in use by the local half of the connection.
    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.conn.local_addr.port()
    }

    /// Returns a map of the response's headers.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Adds or modifies the header field represented by `HeaderName`.
    pub fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    /// Returns the `Header` entry for the given `HeaderName`, if present.
    #[must_use]
    pub fn get_header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Returns the response headers as a String.
    #[must_use]
    pub fn headers_to_string(&self) -> String {
        if self.headers.is_empty() {
            String::new()
        } else {
            self.headers.0.iter().fold(String::new(), 
                |mut acc, (name, value)| {
                    acc.push_str(&format!("{name}: {value}\n"));
                    acc
                })
        }
    }

    /// Returns true if the Connection header is present with the value "close".
    #[must_use]
    pub fn has_close_connection_header(&self) -> bool {
        self.headers.contains(&CONNECTION)
    }

    /// Returns true if a response body is allowed.
    ///
    /// Presence of a response body depends upon the request method and the
    /// response status code.
    #[must_use]
    pub fn body_is_permitted(&self, method: Method) -> bool {
        match self.status_code() {
            // 1xx (Informational), 204 (No Content), and 304 (Not Modified).
            100..=199 | 204 | 304 => false,
            // CONNECT responses with a 2xx (Success) status.
            200..=299 if method == Method::Connect => false,
            // HEAD responses.
            _ if method == Method::Head => false,
            _ => true,
        }
    }

    /// Returns an optional reference to the message body, if present.
    #[must_use]
    pub const fn body(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }

	/// Returns true if the body is unencoded and has a text or application
	/// Content-Type header.
	#[must_use]
    pub fn body_is_printable(&self) -> bool {
        self.headers
            .get(&CONTENT_TYPE)
            .map_or(false,
                |value| {
                    let body_type = value.to_string();
                    body_type.contains("text") || body_type.contains("application")
                })
	}

    /// Returns the response body as a copy-on-write string.
    #[must_use]
    pub fn body_to_string(&self) -> Cow<'_, str> {
        if let Some(body) = self.body.as_ref() {
            if !body.is_empty() && self.body_is_printable() {
                return String::from_utf8_lossy(body);
            }
        }

        String::new().into()
    }

    /// Returns a String representation of the response's status line.
    #[must_use]
    pub fn status_line(&self) -> String {
        self.status_line.to_string()
    }

    /// Writes the `Response` to the underlying TCP connection.
    pub fn send(&mut self) -> IoResult<()> {
        // Status line.
        write!(&mut self.conn.writer, "{}\r\n", self.status_line)?;

        // Response headers.
        self.headers.insert_server();

        for (name, value) in &self.headers.0 {
            write!(&mut self.conn.writer, "{name}: {value}\r\n")?;
        }

        // Mark the end of the headers section.
        self.conn.writer.write_all(b"\r\n")?;

        // Response body.
        if let Some(body) = self.body.as_ref() {
			if !body.is_empty() {
				self.conn.writer.write_all(body)?;
			}
		}

        self.conn.writer.flush()?;
        Ok(())
    }

    /// Reads an HTTP response from a `Connection` and parses it into a `Response`.
    pub fn recv(mut conn: Connection, method: Method) -> NetResult<Self> {
        let mut line = String::new();

        let status_line = match conn.read_line(&mut line) {
            Err(e) => return Err(NetError::ReadError(e.kind())),
            Ok(0) => return Err(IoErrorKind::UnexpectedEof)?,
            Ok(_) => StatusLine::parse(&line)?,
        };

        let mut num_headers = 0;
        let mut headers = Headers::new();

        while num_headers <= MAX_HEADERS {
            line.clear();

            match conn.read_line(&mut line) {
                Err(e) => return Err(NetError::ReadError(e.kind())),
                Ok(0) => return Err(IoErrorKind::UnexpectedEof)?,
                Ok(_) => {
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        break;
                    }

                    let (name, value) = Header::parse(trimmed)?;
                    headers.insert(name, value);

                    num_headers += 1;
                }
            }
        }

        // Parse the response body.
        let maybe_len = headers
            .get(&CONTENT_LENGTH)
            .and_then(
                |len| {
                    let len_str = len.to_string();
                    usize::from_str_radix(&len_str, 10).ok()
                });

        let maybe_type = headers
            .get(&CONTENT_TYPE)
            .map(ToString::to_string);

        let body = {
            if let (Some(ref ctype), Some(clen)) = (maybe_type, maybe_len) {
                Self::parse_body(&mut conn.reader, clen, ctype)
            } else {
                None
            }
        };

        Ok(Self {
            method,
            status_line,
            headers,
            body,
            conn
        })
    }

    /// Reads and parses the message body based on the Content-Type and
    /// Content-Length headers values.
    #[must_use]
    pub fn parse_body(
        reader: &mut NetReader,
        len_val: usize,
        type_val: &str
    ) -> Option<Vec<u8>> {
        let Ok(num_bytes) = u64::try_from(len_val) else {
            return None;
        };

        if !type_val.contains("text") && !type_val.contains("application") {
            return None;
        }

        let mut body = Vec::with_capacity(len_val);
        let mut rdr = reader.take(num_bytes);

        // TODO: handle chunked data and partial reads.
        if rdr.read_to_end(&mut body).is_ok() && !body.is_empty() {
            Some(body)
        } else {
            None
        }
    }
}
