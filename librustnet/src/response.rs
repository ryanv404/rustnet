use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::{BufRead, ErrorKind as IoErrorKind, Read, Result as IoResult, Write};
use std::string::ToString;

use crate::consts::{
    CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, MAX_HEADERS,
};
use crate::{
    Connection, Header, HeaderName, HeaderValue, Headers, Method, NetError,
    NetReader, NetResult, ParseErrorKind, Request, Resolved, Status, Target,
    Version,
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
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = version;
        self
    }

    /// Sets the HTTP response status.
    pub fn status(&mut self, status: Status) -> &mut Self {
        self.status = Some(status);
        self
    }

    /// Sets the HTTP request method.
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = Some(method);
        self
    }

    /// Adds a new header or updates the header value if it is already present.
    pub fn add_header(&mut self, name: HeaderName, value: HeaderValue) -> &mut Self {
        self.headers.insert(name, value);
        self
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Sets the content of the response body.
    pub fn body(&mut self, data: &[u8]) -> &mut Self {
        if data.is_empty() {
            return self;
        }

        self.headers.insert(CONTENT_LENGTH, data.len().into());
        self.headers.insert(CONTENT_TYPE, "text/plain".into());

        self.body = Some(data.to_vec());
        self
    }

    /// Returns a `Response` instance from the `ResponseBuilder`.
    pub fn build(&mut self) -> NetResult<Response> {
        let method = self.method.take().unwrap_or_default();
        let status = self.status.take().unwrap_or_default();
        let status_line = StatusLine::new(self.version, status);
		let headers = self.headers.clone();
		let body = self.body.take();
        let conn = self.conn.try_clone()?;

		Ok(Response {
            conn,
            method,
            status_line,
            headers,
            body,
        })
    }

    /// Sends an HTTP response and then returns the `Response` instance.
    pub fn send(&mut self) -> NetResult<Response> {
        let mut res = self.build()?;
        res.send()?;
        Ok(res)
    }
}

/// Represents the status line of an HTTP response.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
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
        write!(f, "{} {}", &self.version, &self.status)
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
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// Returns the response status.
    #[must_use]
    pub const fn status(&self) -> &Status {
        &self.status
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

    /// Parses a string slice into a `Version` and a `Status` returning a
    /// `StatusLine` object.
    pub fn parse(line: &str) -> NetResult<Self> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Err(ParseErrorKind::StatusLine.into());
        }

        let mut tokens = trimmed.splitn(3, ' ').map(str::trim);
        let parts = (tokens.next(), tokens.next(), tokens.next());

        let (Some(version), Some(status_code), Some(_status_msg)) = parts else {
            return Err(ParseErrorKind::StatusLine.into());
        };

        let Ok(version) = version.parse::<Version>() else {
            return Err(ParseErrorKind::Version.into());
        };

        status_code.parse::<Status>()
            .map_or_else(
                |_| Err(ParseErrorKind::Status.into()),
                |status| Ok(Self::new(version, status))
            )
    }
}

/// Represents the components of an HTTP response.
pub struct Response {
    pub conn: Connection,
    pub method: Method,
    pub status_line: StatusLine,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
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
    /// Returns a new `ResponseBuilder` instance.
    #[must_use]
    #[allow(clippy::new_ret_no_self)]
    pub fn new(conn: Connection) -> ResponseBuilder {
        ResponseBuilder::new(conn)
    }

    /// Parses a `Response` object from a `Request`.
    pub fn from_request(
        req: &Request,
        resolved: &Resolved,
        conn: Connection
    ) -> NetResult<Self> {
        let mut headers = Headers::new();

        let body = match resolved.target() {
            Target::File(ref filepath) => {
                let content = fs::read(filepath)?;

				if content.is_empty() {
					None
				} else {
					let cont_type = HeaderValue::infer_content_type(filepath);
					headers.insert(CONTENT_TYPE, cont_type);
					headers.insert(CONTENT_LENGTH, content.len().into());

					if *req.method() == Method::Head {
						None
					} else {
						Some(content)
					}
				}
            },
            Target::Text(ref text) => Some(text.clone().into_bytes()),
            Target::Bytes(ref bytes) => Some(bytes.to_owned()),
            Target::Empty => None,
        };

        let method = *req.method();
        let status_line = StatusLine::new(*req.version(), resolved.status);

        Ok(Self { conn, method, status_line, headers, body })
    }

    /// Returns the protocol version.
    #[must_use]
    pub const fn version(&self) -> &Version {
        self.status_line.version()
    }

    /// Returns the response's `Status` value.
    #[must_use]
    pub const fn status(&self) -> &Status {
        self.status_line.status()
    }

    /// Returns the status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_line.status_code()
    }

    /// Returns the status reason phrase.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status_line.status_msg()
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
    pub fn set_header(&mut self, name: HeaderName, value: HeaderValue) {
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
        if self.headers.0.is_empty() {
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
        self.headers.get(&CONTENT_TYPE).map_or(false,
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
        let mut num_headers = 0;
        let mut line = String::new();
        let mut headers = Headers::new();

        let status_line = match conn.read_line(&mut line) {
            Err(e) => return Err(NetError::ReadError(e.kind())),
            Ok(0) => {
                return Err(NetError::ReadError(IoErrorKind::UnexpectedEof));
            },
            Ok(_) => StatusLine::parse(&line)?,
        };

        while num_headers <= MAX_HEADERS {
            line.clear();

            match conn.read_line(&mut line) {
                Err(e) => return Err(NetError::ReadError(e.kind())),
                Ok(0) => {
                    return Err(NetError::ReadError(IoErrorKind::UnexpectedEof));
                },
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
            .and_then(|len| {
                let len_str = len.to_string();
                len_str.parse::<usize>().ok()
            });

        let maybe_type = headers
            .get(&CONTENT_TYPE)
            .map(ToString::to_string);

        let body = {
            if let (Some(ref ctype), Some(clen)) = (maybe_type, maybe_len) {
                Self::parse_body(&mut conn.reader, clen, ctype.as_str())
            } else {
                None
            }
        };

        Ok(Self { conn, method, status_line, headers, body })
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
        if rdr.read_to_end(&mut body).is_ok() && !body.is_empty() {
            Some(body)
        } else {
            None
        }
    }
}
