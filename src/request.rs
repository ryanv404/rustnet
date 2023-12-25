use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::str::{self, FromStr};

use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetError,
    NetParseError, NetResult, Route, Version,
};
use crate::colors::{CLR, YLW};
use crate::util;

/// Contains the components of an HTTP request line.
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
            version: Version::OneDotOne,
        }
    }
}

impl Display for RequestLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {} {}", &self.method, &self.path, &self.version)
    }
}

impl TryFrom<&[u8]> for RequestLine {
    type Error = NetError;

    fn try_from(line: &[u8]) -> NetResult<Self> {
        let line = util::trim_whitespace_bytes(line);

        let mut tokens = line.splitn(3, |b| *b == b' ');

        let method = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::Method))
            .and_then(Method::try_from)?;

        let path = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::Path))
            .and_then(|token| String::from_utf8(token.to_vec())
                .map_err(|_| NetError::Parse(NetParseError::Path)))?;

        let version = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::Version))
            .and_then(Version::try_from)?;

        Ok(Self { method, path, version })
    }
}

impl FromStr for RequestLine {
    type Err = NetError;

    fn from_str(line: &str) -> NetResult<Self> {
        Self::try_from(line.as_bytes())
    }
}

impl RequestLine {
    /// Returns a new `RequestLine` instance from the provided HTTP method
    /// and URI path.
    #[must_use]
    pub fn new(method: &Method, path: &str) -> Self {
        Self {
            method: method.clone(),
            path: path.to_string(),
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
        &self.path
    }

    /// Returns a reference to the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// Returns the `RequestLine` as a `String` with color formatting.
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

impl Request {
    /// Returns a reference to the HTTP `Method`.
    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.request_line.method
    }

    /// Returns the requested URI path as a string slice.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.request_line.path
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
