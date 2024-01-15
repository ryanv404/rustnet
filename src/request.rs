use std::borrow::{Borrow, Cow};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::iter;
use std::str::{self, FromStr};

use crate::{Body, Headers, Method, NetError, NetResult, Version};
use crate::headers::names::CONTENT_TYPE;
use crate::style::colors::{ORANGE, RESET};
use crate::utils;

/// An HTTP request builder object.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestBuilder {
    pub method: Method,
    pub path: UriPath,
    pub version: Version,
    pub headers: Headers,
    pub body: Body,
}

impl RequestBuilder {
    /// Returns a new `RequestBuilder` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP method.
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = method;
        self
    }

    /// Sets the URI path.
    pub fn path(&mut self, path: UriPath) -> &mut Self {
        self.path = path;
        self
    }

    /// Sets the HTTP protocol version.
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = version;
        self
    }

    /// Inserts a header entry from the given name and value.
    pub fn header(&mut self, name: &str, value: &[u8]) -> &mut Self {
        self.headers.header(name, value);
        self
    }

    /// Appends the header entries from `other`.
    pub fn headers(&mut self, mut other: Headers) -> &mut Self {
        self.headers.append(&mut other);
        self
    }

    /// Sets the request body.
    pub fn body(&mut self, body: Body) -> &mut Self {
        self.body = body;
        self
    }

    /// Builds and returns a new `Request` instance.
    pub fn build(&mut self) -> Request {
        // Ensure the default request headers are set.
        self.headers.default_request_headers(&self.body, None);

        Request {
            method: self.method,
            path: self.path.clone(),
            version: self.version,
            headers: self.headers.clone(),
            body: self.body.clone()
        }
    }
}

/// The path component of an HTTP URI.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct UriPath(pub Cow<'static, str>);

impl Default for UriPath {
    fn default() -> Self {
        Self(Cow::Borrowed("/"))
    }
}

impl Display for UriPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for UriPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "UriPath({:?})", self.as_str())
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
    type Error = NetError;

    fn try_from(bytes: &'static [u8]) -> NetResult<Self> {
        str::from_utf8(bytes)
            .map_err(|_| NetError::BadPath)
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
}

/// Contains the components of an HTTP request.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

        writeln!(f, "{}", &self.headers)?;

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

    fn try_from(req: &[u8]) -> NetResult<Self> {
        let mut lines = utils::trim_start(req).split(|&b| b == b'\n');

        let (method, path, version) = lines
            .next()
            .ok_or(NetError::BadRequest)
            .and_then(Self::parse_request_line)?;

        let headers_bytes = lines
            .by_ref()
            .map_while(|line| {
                let line = utils::trim(line);

                if line.is_empty() {
                    None
                } else {
                    Some(line)
                }
            })
            // Restore newline characters removed by `split` above.
            .flat_map(|line| line.iter().copied().chain(iter::once(b'\n')))
            .collect::<Vec<u8>>();

        let headers = Headers::try_from(&headers_bytes[..])?;

        let content_type = headers
            .get(&CONTENT_TYPE)
            .map_or(Cow::Borrowed(""), |value| value.as_str());

        let body_bytes = lines
            // Restore newline characters removed by `split` above.
            .flat_map(|line| line.iter().copied().chain(iter::once(b'\n')))
            .collect::<Vec<u8>>();

        let body = Body::from_content_type(&body_bytes, &content_type);

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

    /// Parses a bytes slice into a `Method`, `UriPath`, and `Version`.
    ///
    /// # Errors
    ///
    /// Retuns an error if parsing of the request line fails.
    pub fn parse_request_line(
        line: &[u8]
    ) -> NetResult<(Method, UriPath, Version)> {
        let mut parts = utils::trim_start(line).split(|&b| b == b' ');

        let method = parts
            .next()
            .map(utils::trim_end)
            .ok_or(NetError::BadMethod)
            .and_then(Method::try_from)?;

        let path = parts
            .next()
            .map(utils::trim)
            .ok_or(NetError::BadPath)
            .and_then(|path| {
                String::from_utf8(path.to_vec())
                    .map_err(|_| NetError::BadPath)
                    .map(UriPath::from)
            })?;

        let version = parts
            .next()
            .map(utils::trim)
            .ok_or(NetError::BadVersion)
            .and_then(Version::try_from)?;

        Ok((method, path, version))
    }

    /// Returns the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
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

    /// Returns the request line as a `String` with plain formatting.
    #[must_use]
    pub fn request_line_to_plain_string(&self) -> String {
        format!("{} {} {}", &self.method, &self.path, &self.version)
    }

    /// Returns the request line as a `String` with color formatting.
    #[must_use]
    pub fn request_line_to_color_string(&self) -> String {
        format!(
            "{ORANGE}{} {} {}{RESET}",
            &self.method,
            &self.path,
            &self.version
        )
    }

    /// Returns a reference to the request `Headers`.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns a reference to the request `Body`.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }
}
