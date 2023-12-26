use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::iter;
use std::str::{self, FromStr};

use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetError,
    NetParseError, NetResult, Route, Version,
};
use crate::colors::{CLR, YLW};
use crate::header_name::CONTENT_TYPE;
use crate::util;

/// An HTTP request builder object.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestBuilder {
    pub method: Option<Method>,
    pub path: Option<Path>,
    pub version: Option<Version>,
    pub headers: Option<Headers>,
    pub body: Option<Body>,
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self {
            method: None,
            path: None,
            version: None,
            headers: None,
            body: None
        }
    }
}

impl RequestBuilder {
    /// Returns a new `RequestBuilder` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP method.
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = Some(method);
        self
    }

    /// Sets the HTTP version.
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    /// Sets the URI path to the target resource.
    pub fn path(&mut self, path: &str) -> &mut Self {
        if !path.is_empty() {
            self.path = Some(path.into());
        }

        self
    }

    /// Inserts a request header.
    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        if self.headers.is_none() {
            self.headers = Some(Headers::new());
        }

        if let Some(headers) = self.headers.as_mut() {
            headers.header(name, value);
        }

        self
    }

    /// Sets the request headers.
    pub fn headers(&mut self, headers: Headers) -> &mut Self {
        self.headers = Some(headers);
        self
    }

    /// Sets the request body.
    pub fn body(&mut self, body: Body) -> &mut Self {
        if body.is_empty() {
            self.body = Some(Body::Empty);
        } else {
            self.body = Some(body);
        }

        self
    }

    /// Builds and returns a new `Request`.
    pub fn build(&mut self) -> Request {
        let request_line = RequestLine {
            method: self.method.take().unwrap_or_default(),
            path: self.path.take().unwrap_or_default(),
            version: self.version.take().unwrap_or_default()
        };

        let headers = self.headers.take().unwrap_or_default();
        let body = self.body.take().unwrap_or_default();

        Request { request_line, headers, body }
    }
}

/// The path component of an HTTP URI.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Path(pub String);

impl Default for Path {
    fn default() -> Self {
        Self(String::from("/"))
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.0)
    }
}

impl TryFrom<&[u8]> for Path {
    type Error = NetError;

    fn try_from(bytes: &[u8]) -> NetResult<Self> {
        let inner = String::from_utf8(bytes.to_vec())
            .map_err(|_| NetError::Parse(NetParseError::Path))?;

        Ok(Self(inner))
    }
}

impl From<&str> for Path {
    fn from(path: &str) -> Self {
        Self(path.to_string())
    }
}

impl Path {
    /// Returns the path as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Contains the components of an HTTP request line.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestLine {
    pub method: Method,
    pub path: Path,
    pub version: Version,
}

impl Display for RequestLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {} {}", &self.method, &self.path, &self.version)
    }
}

impl FromStr for RequestLine {
    type Err = NetError;

    fn from_str(line: &str) -> NetResult<Self> {
        Self::try_from(line.as_bytes())
    }
}

impl TryFrom<&[u8]> for RequestLine {
    type Error = NetError;

    fn try_from(line: &[u8]) -> NetResult<Self> {
        let line = util::trim_bytes(line);

        let mut tokens = line.splitn(3, |b| *b == b' ');

        let method = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::Method))
            .and_then(Method::try_from)?;

        let path = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::Path))
            .and_then(Path::try_from)?;

        let version = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::Version))
            .and_then(Version::try_from)?;

        Ok(Self { method, path, version })
    }
}

impl RequestLine {
    /// Returns a new `RequestLine` instance from the provided HTTP method
    /// and URI path.
    #[must_use]
    pub fn new(method: &Method, path: &str) -> Self {
        Self {
            method: method.clone(),
            path: path.into(),
            version: Version::OneDotOne,
        }
    }

    /// Returns a reference to the HTTP `Method`.
    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.method
    }

    /// Returns the requested URI path as a string slice.
    #[must_use]
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    /// Returns a reference to the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// Returns the `RequestLine` as a `String` with color formatting.
    #[must_use]
    pub fn to_color_string(&self) -> String {
        format!("{YLW}{self}{CLR}")
    }
}

/// Contains the components of an HTTP request.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Request {
    pub request_line: RequestLine,
    pub headers: Headers,
    pub body: Body,
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{}", &self.request_line)?;

        for (name, value) in &self.headers.0 {
            writeln!(f, "{name}: {value}")?;
        }

        if self.body.is_printable() {
            writeln!(f, "{}", &self.body)?;
        }

        Ok(())
    }
}

impl FromStr for Request {
    type Err = NetError;

    fn from_str(req: &str) -> NetResult<Self> {
        Self::try_from(req.as_bytes())
    }
}

impl TryFrom<&[u8]> for Request {
    type Error = NetError;

    fn try_from(bytes: &[u8]) -> NetResult<Self> {
        let trimmed = util::trim_start_bytes(bytes);

        let mut lines = trimmed.split(|b| *b == b'\n');

        // Parse the RequestLine.
        let request_line = lines
            .next()
            .ok_or(NetError::Parse(NetParseError::RequestLine))
            .and_then(RequestLine::try_from)?;

        let mut headers = Headers::new();

        // Collect the trimmed header lines into a new iterator.
        let header_lines = lines
            .by_ref()
            .map_while(|line| {
                let trimmed = util::trim_bytes(line);

                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            });

        // Parse and insert each header.
        for line in header_lines {
            headers.insert_parsed_header_bytes(line)?;
        }

        // Collect the remaining bytes while restoring the newlines that were
        // removed from each line due to the call to `split` above.
        let body_bytes = lines
            .flat_map(|line| line
                .iter()
                .copied()
                .chain(iter::once(b'\n'))
            )
            .collect::<Vec<u8>>();

        // Determine `Body` type using the Content-Type header if present.
        let content_type = headers
            .get(&CONTENT_TYPE)
            .map_or(Cow::Borrowed(""), |value| value.as_str());

        let body = if content_type.is_empty() {
            Body::Empty
        } else {
            Body::from_content_type(&body_bytes, &content_type)
        };

        Ok(Self { request_line, headers, body })
    }
}

impl Request {
    /// Returns a reference to the HTTP `Method`.
    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.request_line.method
    }

    /// Returns the requested URI path as a string slice.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.request_line.path.as_str()
    }

    /// Returns a reference to the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.request_line.version
    }

    /// Returns a `Route` which represents the request `Method` and URI path.
    #[must_use]
    pub fn route(&self) -> Route {
        Route::new(self.method(), self.path())
    }

    /// Returns a reference to the `RequestLine`.
    #[must_use]
    pub const fn request_line(&self) -> &RequestLine {
        &self.request_line
    }

    /// Returns a reference to the request headers.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the header with the given `HeaderName` key is present.
    #[must_use]
    pub fn contains(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Adds a header to this `Request`.
    pub fn header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    /// Returns a reference to the request `Body`.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }
}
