use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufWriter, StdoutLock, Write};
use std::str::{self, FromStr};
use std::string::ToString;

use crate::{
    Body, Connection, HeaderName, HeaderValue, Headers, Method, NetError,
    NetParseError, NetResult, Route, Version,
};

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

impl FromStr for RequestLine {
    type Err = NetError;

    fn from_str(line: &str) -> NetResult<Self> {
        let mut tokens = line.trim().splitn(3, ' ');

        let method = tokens
            .next()
            .ok_or::<NetError>(NetParseError::Method.into())
            .map(Into::into)?;

        let path = tokens
            .next()
            .map(ToString::to_string)
            .ok_or(NetParseError::Path)?;

        let version = tokens
            .next()
            .ok_or::<NetError>(NetParseError::Version.into())
            .and_then(Version::from_str)?;

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
            path: path.to_string(),
            version: Version::OneDotOne,
        }
    }

    /// Returns a reference to the HTTP `Method`.
    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.method
    }

    /// Returns the requested URI path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns a reference to the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// Writes the `RequestLine` to a `BufWriter` with plain formatting.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the provided `BufWriter` fails.
    pub fn write_plain(
        &self,
        writer: &mut BufWriter<StdoutLock<'_>>
    ) -> NetResult<()> {
        writeln!(writer, "{self}")?;
        Ok(())
    }

    /// Writes the `RequestLine` to a `BufWriter` with color formatting.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the provided `BufWriter` fails.
    pub fn write_color(
        &self,
        writer: &mut BufWriter<StdoutLock<'_>>
    ) -> NetResult<()> {
        use crate::colors::{CLR, YLW};

        writeln!(writer, "{YLW}{self}{CLR}")?;
        Ok(())
    }
}

/// Contains the components of an HTTP request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
            let body = String::from_utf8_lossy(self.body.as_bytes());
            writeln!(f, "{body}")?;
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

    /// Writes an HTTP request to a `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::send_request` fails.
    pub fn send(&mut self, conn: &mut Connection) -> NetResult<()> {
        conn.send_request(self)
    }

    /// Reads and parses an HTTP request from a `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::recv_request` fails.
    pub fn recv(conn: &mut Connection) -> NetResult<Self> {
        conn.recv_request()
    }
}
