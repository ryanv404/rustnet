use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufRead, ErrorKind as IoErrorKind, Read};
use std::str;
use std::string::ToString;

use crate::consts::{CONTENT_LENGTH, CONTENT_TYPE, MAX_HEADERS};
use crate::{
    Connection, Header, HeaderName, HeaderValue, Headers, Method, NetError, 
    NetReader, NetResult, ParseErrorKind, Route, Version,
};

/// Represents the first line of an HTTP request.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
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
    #[must_use]
    pub const fn new(method: Method, path: String, version: Version) -> Self {
        Self { method, path, version }
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

    /// Parses a string slice into a `RequestLine` object.
    pub fn parse(line: &str) -> NetResult<Self> {
        let mut tokens = line.trim_start().splitn(3, ' ');

        let method = Method::parse(tokens.next())?;

        let path = tokens.next()
            .ok_or(ParseErrorKind::RequestLine)
            .map(ToString::to_string)?;

        let version = Version::parse(tokens.next())?;

        Ok(Self { method, path, version })
    }
}

/// Represents the components of an HTTP request.
pub struct Request {
    pub request_line: RequestLine,
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
    pub conn: Connection,
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		// The request line.
		writeln!(f, "{}", self.request_line)?;

		// The request headers.
		for (name, value) in &self.headers.0 {
			writeln!(f, "{name}: {value}")?;
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
			.field("request_line", &self.request_line)
			.field("headers", &self.headers)
            .field("body", &self.body.as_ref().and_then(
                |body| if self.body_is_printable() {
                    Some(body)
                } else {
                    None
                }))
			.field("conn", &self.conn)
            .finish()
    }
}

impl TryFrom<Connection> for Request {
    type Error = NetError;

    fn try_from(mut conn: Connection) -> NetResult<Self> {
        // Reads an HTTP request and parses it into a `Request` object.
        let mut line = String::new();

        // Parse the request line.
        let request_line = match conn.reader.read_line(&mut line) {
            Err(e) => return Err(NetError::ReadError(e.kind())),
            Ok(0) => return Err(IoErrorKind::UnexpectedEof)?,
            Ok(_) => RequestLine::parse(&line)?,
        };

        let mut header_num = 0;
        let mut headers = Headers::new();

        // Parse the request headers.
        while header_num <= MAX_HEADERS {
            line.clear();

            match conn.reader.read_line(&mut line) {
                Err(e) => return Err(NetError::ReadError(e.kind())),
                Ok(0) => return Err(IoErrorKind::UnexpectedEof)?,
                Ok(_) => {
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        break;
                    }

                    let (hdr_name, hdr_value) = Header::parse(trimmed)?;
                    headers.insert(hdr_name, hdr_value);

                    header_num += 1;
                }
            }
        }

        // Parse the request body.
        let maybe_len = headers
            .get(&CONTENT_LENGTH)
            .and_then(
                |len| {
                    let len_str = len.to_string();
                    len_str.parse::<usize>().ok()
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

        Ok(Self { request_line, headers, body, conn })
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

    /// Returns a reference to the request headers.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Returns the header value for the given `HeaderName`, if present.
    #[must_use]
    pub fn get_header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

	/// Adds or updates a request header field line.
    pub fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    /// Returns the request headers as a String.
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

	/// Returns true if the body has a text/* or application/* Content-Type header.
    #[must_use]
    pub fn body_is_printable(&self) -> bool {
        self.headers.get(&CONTENT_TYPE).map_or(false,
            |val| {
                let body_type = val.to_string();
                body_type.contains("text") || body_type.contains("application")
            })
    }

	/// Returns a reference to the request body, if present.
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

    /// Uses header values to read the message body.
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
