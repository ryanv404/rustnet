use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::ErrorKind as IoErrorKind;
use std::str;
use std::string::ToString;

use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetReader,
    NetResult, NetWriter, ParseErrorKind, Route, Version,
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
    pub body: Body,
    pub reader: Option<NetReader>,
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		// The request line.
		writeln!(f, "{}", self.request_line)?;

		// The request headers.
		for (name, value) in &self.headers.0 {
			writeln!(f, "{name}: {value}")?;
		}

        // The request body.
        if !self.body.is_empty() {
            writeln!(f, "{}", &self.body)?;
        }

		Ok(())
    }
}

impl Debug for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("Request")
			.field("request_line", &self.request_line)
			.field("headers", &self.headers)
            .field("body", &self.body)
			.field("reader", &self.reader)
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

	/// Returns a reference to the request body, if present.
	#[must_use]
	pub const fn body(&self) -> &Body {
		&self.body
	}

    /// Sends an HTTP request to a remote server.
    pub fn send(&mut self) -> NetResult<()> {
        let mut writer = self.reader
            .as_ref()
            .and_then(|reader| NetWriter::try_from(reader).ok())
            .ok_or_else(|| IoErrorKind::NotConnected)?;

        writer.send_request(self)
    }

    /// Receives an HTTP request from a remote client.
    pub fn recv(mut reader: NetReader) -> NetResult<Request> {
        reader.recv_request()
    }
}
