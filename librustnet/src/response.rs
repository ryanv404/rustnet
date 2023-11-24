use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::{BufRead, ErrorKind as IoErrorKind, Read, Result as IoResult, Write};

use crate::consts::{
    CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, DEFAULT_NAME, MAX_HEADERS, SERVER,
};
use crate::{
    Connection, Header, HeaderName, HeaderValue, HeadersSet, Method, NetError,
    NetReader, NetResult, ParseErrorKind, Request, Resolved, Status, Target,
    Version,
};

/// An HTTP response builder object.
#[derive(Debug)]
pub struct ResponseBuilder {
    pub conn: Option<Connection>,
    pub method: Option<Method>,
    pub version: Option<Version>,
    pub status: Option<Status>,
    pub headers: HeadersSet,
    pub body: Option<Vec<u8>>,
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self {
            conn: None,
            method: None,
            version: None,
            status: None,
            headers: BTreeSet::<Header>::new(),
            body: None,
        }
    }
}

impl ResponseBuilder {
    /// Returns a `ResponseBuilder instance`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the connection object that represents the underlying TCP connection.
    pub fn conn(&mut self, conn: Connection) -> &mut Self {
        self.conn = Some(conn);
        self
    }

    /// Sets the protocol version.
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
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
        let header = Header::new(name, value);
        if self.headers.is_empty() {
            self.headers = BTreeSet::<Header>::from([header]);
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

    /// Sets the content of the response body.
    pub fn body(&mut self, data: &[u8]) -> &mut Self {
        if !data.is_empty() {
            self.body = Some(data.to_vec());
        }

        self
    }

    /// Returns a `Response` instance from the `ResponseBuilder`.
    pub fn build(&mut self) -> IoResult<Response> {
        let conn = self.conn.take();
        let method = self.method.take().unwrap_or_default();

        let version = self.version.take().unwrap_or_default();
        let status = self.status.take().unwrap_or_default();
        let status_line = StatusLine::new(version, status);

		let headers = self.headers;
		let body = self.body.take();

		Ok(Response {
            conn,
            method,
            status_line,
            headers,
            body,
        })
    }

    /// Sends an HTTP response and then returns the `Response` instance.
    pub fn send(&mut self) -> IoResult<Response> {
        let mut res = self.build()?;
        res.send()?;
        Ok(res)
    }

    /// Receives an HTTP response and returns it as a `Response` instance.
    pub fn recv(&mut self) -> NetResult<Response> {
        todo!();
    }
}

/// Represents the status line of an HTTP response.
#[derive(Debug)]
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
    pub fn new(version: Version, status: Status) -> Self {
        Self { version, status }
    }

    /// Returns the protocol version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
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

        if let Ok(status) = status_code.parse::<Status>() {
            Ok(Self::new(version, status))
        } else {
            Err(ParseErrorKind::Status.into())
        }
    }
}

/// Represents the components of an HTTP response.
pub struct Response {
    pub conn: Option<Connection>,
    pub method: Method,
    pub status_line: StatusLine,
    pub headers: HeadersSet,
    pub body: Option<Vec<u8>>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            conn: None,
            method: Method::default(),
            status_line: StatusLine::default(),
            headers: BTreeSet::<Header>::new(),
            body: None,
        }
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // The response status line.
        writeln!(f, "{}", self.status_line)?;

        // The response headers.
        for header in self.headers.iter() {
            writeln!(f, "{header}")?;
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
		    .field("conn", &"Connection { ... }")
			.field("method", &self.method)
			.field("status_line", &self.status_line)
			.field("headers", &self.headers)
            .field("body", &self.body)
			.finish()
	}
}

impl Response {
    /// Parses a `Response` object from a `Request`.
    pub fn from_request(mut req: Request, resolved: &Resolved) -> NetResult<Self> {
        let mut headers = BTreeSet::<Header>::new();

        let body = match resolved.target() {
            Target::File(ref filepath) => {
                let content = fs::read(filepath)?;

				if content.is_empty() {
					None
				} else {
					let cont_type = HeaderValue::infer_content_type(filepath);
					headers.insert(Header::new(CONTENT_TYPE, cont_type));
					headers.insert(Header::new(CONTENT_LENGTH, content.len().into()));

					if req.method() == Method::Head {
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

        let conn = req.conn.take();
        let method = req.method();
        let status_line = StatusLine::new(req.version(), resolved.status);

        Ok(Self { conn, method, status_line, headers, body })
    }

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
        self.status_line.status_code()
    }

    /// Returns the status reason phrase.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status_line.status_msg()
    }

    /// Returns a map of the response's headers.
    #[must_use]
    pub const fn headers(&self) -> Option<&HeadersSet> {
        &self.headers
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Adds or modifies the header field represented by `HeaderName`.
    pub fn set_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(Header::new(name, value));
    }

    /// Returns the `Header` entry for the given `HeaderName`, if present.
    #[must_use]
    pub fn header(&self, name: &HeaderName) -> Option<&Header> {
        self.headers.get(name)
    }

    /// Returns all of the response headers as a String.
	#[must_use]
    pub fn headers_to_string(&self) -> String {
        if self.headers.is_none() {
            String::new()
        } else {
            self.headers
                .as_ref()
                .map_or(String::new(),
                    |headers| headers
                        .iter()
                        .fold(String::new(), |mut acc, header| {
                            acc.push_str(&format!("{header}\n"));
                            acc
                        })
                )
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
        if let Some(header) = self.headers.get(&CONTENT_TYPE) {
            let body_type = header.value.to_string();
    		body_type.contains("text") || body_type.contains("application")
        } else {
            false
        }
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

    /// Writes the response's status line to a stream.
    pub fn write_status_line(&mut self) -> IoResult<()> {
        let writer = self
            .conn
            .as_mut()
            .map(|conn| conn.writer.by_ref())
            .ok_or(NetError::WriteError(IoErrorKind::NotConnected))?;

        write!(writer, "{}\r\n", self.status_line)?;
        writer.flush()?;
        Ok(())
    }

    /// Writes the response's headers to a stream.
    pub fn write_headers(&mut self) -> IoResult<()> {
        self.set_header(SERVER, HeaderValue::from(DEFAULT_NAME));

        let writer = self
            .conn
            .as_mut()
            .map(|conn| conn.writer.by_ref())
            .ok_or(NetError::WriteError(IoErrorKind::NotConnected))?;

        self.headers.iter().for_each(|header| {
            write!(writer, "{header}\r\n").unwrap();
        });

        // Mark the end of the headers section.
        writer.write_all(b"\r\n")?;
        writer.flush()?;
        Ok(())
    }

    /// Writes the response's body to a stream, if applicable.
    pub fn write_body(&mut self) -> IoResult<()> {
        let writer = self
            .conn
            .as_mut()
            .map(|conn| conn.writer.by_ref())
            .ok_or(NetError::WriteError(IoErrorKind::NotConnected))?;

        if let Some(body) = self.body.as_ref() {
			if !body.is_empty() {
				writer.write_all(body)?;
			}
		}

        writer.flush()?;
        Ok(())
    }

    /// Writes the `Response` to the underlying TCP connection.
    pub fn send(&mut self) -> IoResult<()> {
        self.write_status_line()?;
        self.write_headers()?;
        self.write_body()?;
        Ok(())    
    }

    /// Reads an HTTP response from a `Connection` and parses it into a `Response`.
    pub fn recv(mut conn: Connection, method: Method) -> NetResult<Self> {
        let mut num_headers = 0;
        let mut line = String::new();
        let mut headers = BTreeSet::<Header>::new();

        let status_line = match conn.read_line(&mut line) {
            Err(e) => return Err(e.into()),
            Ok(0) => return Err(NetError::UnexpectedEof.into()),
            Ok(_) => StatusLine::parse(&line)?,
        };

        while num_headers <= MAX_HEADERS {
            line.clear();

            match conn.read_line(&mut line) {
                Err(e) => return Err(e.into()),
                Ok(0) => return Err(NetError::UnexpectedEof.into()),
                Ok(_) => {
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        break;
                    }

                    let header = Header::parse(trimmed)?;
                    headers.insert(header);
                    num_headers += 1;
                }
            }
        }

        // Only parse the body if Content-Length and Content-Type headers are present.
        let body_len = headers
            .get(&CONTENT_LENGTH)
            .and_then(|cont_len| {
                let len_str = cont_len.value.to_string();
                len_str.parse::<usize>().ok()
            });

        let body_type = headers
            .get(&CONTENT_TYPE)
            .and_then(|cont_type| Some(cont_type.value.to_string()));

        let body = Self::parse_body(&mut conn.reader, body_len, body_type.as_ref());

        let conn = Some(conn);
        let headers = headers;

        Ok(Self { conn, method, status_line, headers, body })
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
}
