use std::borrow::{Borrow, Cow};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::iter;
use std::str::{self, FromStr};

use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetParseError, Route,
    Version, DEFAULT_NAME,
};
use crate::header::names::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, USER_AGENT};
use crate::style::colors::{BR_YLW, CLR};
use crate::util;

/// An HTTP request builder object.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestBuilder {
    pub method: Option<Method>,
    pub path: Option<UriPath>,
    pub version: Option<Version>,
    pub headers: Option<Headers>,
    pub body: Option<Body>,
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

    /// Sets the URI path.
    pub fn path(&mut self, path: UriPath) -> &mut Self {
        self.path = Some(path);
        self
    }

    /// Sets the HTTP version.
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    /// Inserts a request header.
    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        if let Some(headers) = self.headers.as_mut() {
            headers.header(name, value);
        } else {
            let mut headers = Headers::default();
            headers.header(name, value);
            self.headers = Some(headers);
        }

        self
    }

    /// Sets the request headers.
    pub fn headers(&mut self, mut headers: Headers) -> &mut Self {
        match self.headers.as_mut() {
            Some(hdrs) => hdrs.append(&mut headers),
            None => self.headers = Some(headers),
        }

        self
    }

    /// Sets the request body.
    pub fn body(&mut self, body: Body) -> &mut Self {
        if !body.is_empty() {
            self.body = Some(body);
        }

        self
    }

    /// Builds and returns a new `Request`.
    pub fn build(&mut self) -> Request {
        let mut req = Request {
            method: self.method.take().unwrap_or_default(),
            path: self.path.take().unwrap_or_default(),
            version: self.version.take().unwrap_or_default(),
            headers: self.headers.take().unwrap_or_default(),
            body: self.body.take().unwrap_or_default()
        };

        // Ensure Accept header is set.
        if !req.headers.contains(&ACCEPT) {
            req.headers.accept("*/*");
        }

        // Ensure User-Agent header is set.
        if !req.headers.contains(&USER_AGENT) {
            req.headers.user_agent(DEFAULT_NAME);
        }

        if !req.body.is_empty() {
            // Ensure Content-Length header is set.
            if !req.headers.contains(&CONTENT_LENGTH) {
                req.headers.content_length(req.body.len());
            }

            // Ensure Content-Type header is set.
            if !req.headers.contains(&CONTENT_TYPE) {
                if let Some(cont_type) = req.body.as_content_type() {
                    req.headers.content_type(cont_type);
                }
            }
        }

        req
    }
}

/// The path component of an HTTP URI.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UriPath(pub Cow<'static, str>);

impl Default for UriPath {
    fn default() -> Self {
        Self(Cow::Borrowed("/"))
    }
}

impl Display for UriPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", &self.0)
    }
}

impl From<&'static str> for UriPath {
    fn from(path: &'static str) -> Self {
        Self(Cow::Borrowed(path))
    }
}

impl From<String> for UriPath {
    fn from(path: String) -> Self {
        Self(Cow::Owned(path))
    }
}

impl TryFrom<&'static [u8]> for UriPath {
    type Error = NetParseError;

    fn try_from(bytes: &'static [u8]) -> Result<Self, Self::Error> {
        str::from_utf8(bytes)
            .map_err(|_| NetParseError::Path)
            .map(Into::into)
    }
}

impl UriPath {
    /// Returns the URI path as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.borrow()
    }

    /// Returns the URI path as a bytes slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    /// Returns true if this `UriPath` is the default path ("/").
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.as_str() == "/"
    }
}

/// Contains the components of an HTTP request.
#[derive(Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Request {
    pub method: Method,
    pub path: UriPath,
    pub version: Version,
    pub headers: Headers,
    pub body: Body,
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{} {} {}", &self.method, &self.path, &self.version)?;

        for (name, value) in &self.headers.0 {
            writeln!(f, "{name}: {value}")?;
        }

        if self.body.is_printable() {
            writeln!(f, "{}", &self.body)?;
        }

        Ok(())
    }
}

impl Debug for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "Request {{")?;
        writeln!(f, "    method: {:?},", &self.method)?;
        writeln!(f, "    path: {:?},", &self.path)?;
        writeln!(f, "    version: {:?}", &self.version)?;
        writeln!(f, "    headers: Headers(")?;
        for (name, value) in &self.headers.0 {
            write!(f, "        ")?;
            writeln!(f, "{name:?}: {value:?},")?;
        }
        writeln!(f, "    ),")?;
        if self.body.is_empty() {
            writeln!(f, "    body: Body::Empty")?;
        } else if self.body.is_printable() {
            writeln!(f, "    body: {:?}", &self.body)?;
        } else {
            writeln!(f, "    body: Body {{ ... }}")?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl FromStr for Request {
    type Err = NetParseError;

    fn from_str(req: &str) -> Result<Self, Self::Err> {
        Self::try_from(req.as_bytes())
    }
}

impl TryFrom<&[u8]> for Request {
    type Error = NetParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let trimmed = util::trim_start(bytes);

        let mut lines = trimmed.split(|b| *b == b'\n');

        // Parse the request line.
        let line = lines
            .next()
            .ok_or(NetParseError::RequestLine)?;

        let line_str = str::from_utf8(line)
            .map_err(|_| NetParseError::RequestLine)?;

        let (method, path, version) = Self::parse_request_line(line_str)?;

        let mut headers = Headers::new();

        // Collect the trimmed header lines into a new iterator.
        let header_lines = lines
            .by_ref()
            .map_while(|line| {
                let trimmed = util::trim(line);

                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            });

        // Parse and insert each header.
        for line in header_lines {
            headers.insert_header_from_bytes(line)?;
        }

        // Collect the remaining bytes while restoring the newlines that were
        // removed from each line due to the call to `split` above.
        let body_vec = lines
            .flat_map(|line| line
                .iter()
                .copied()
                .chain(iter::once(b'\n'))
            )
            .collect::<Vec<u8>>();

        // Parse the `Body` using the Content-Type header.
        let content_type = headers
            .get(&CONTENT_TYPE)
            .map(|value| value.as_str())
            .unwrap_or(Cow::Borrowed(""));

        let body = Body::from_content_type(&body_vec, &content_type);

        Ok(Self { method, path, version, headers, body })
    }
}

impl Request {
    /// Returns a new `RequestBuilder` instance.
    #[must_use]
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    /// Returns a default `Request` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the HTTP `Method`.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the requested URI path as a string slice.
    #[must_use]
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    /// Returns the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Returns a `Route` which represents the request `Method` and URI path.
    #[must_use]
    pub fn route(&self) -> Route {
        Route::new(self.method(), self.path().to_string())
    }

    /// Returns the request line as a `String` with plain formatting.
    #[must_use]
    pub fn request_line_to_plain_string(&self) -> String {
        format!("{} {} {}", &self.method, &self.path, &self.version)
    }

    /// Returns the request line as a `String` with color formatting.
    #[must_use]
    pub fn request_line_to_color_string(&self) -> String {
        format!(
            "{BR_YLW}{} {} {}{CLR}",
            &self.method,
            &self.path,
            &self.version
        )
    }

    /// Parses a string slice into a `Method`, `UriPath`, and `Version.
    pub fn parse_request_line(
        line: &str
    ) -> Result<(Method, UriPath, Version), NetParseError> {
        let (method, remaining) = line
            .trim_start()
            .split_once(' ')
            .ok_or(NetParseError::RequestLine)?;

        remaining
            .trim_start()
            .split_once(' ')
            .ok_or(NetParseError::RequestLine)
            .and_then(|(path, version)| {
                let method = Method::from_str(method.trim_end())?;
                let path = path.trim_end().to_string().into();
                let version = Version::from_str(version.trim())?;
                Ok((method, path, version))
            })
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
