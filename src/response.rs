use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::str::{self, FromStr};

use crate::{
    Body, Header, Headers, NetError, NetResult, Status, Target, Version,
    utils,
};
use crate::headers::names::CONTENT_TYPE;
use crate::style::colors::{MAGENTA, RESET};

/// An HTTP response builder object.
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResponseBuilder {
    pub version: Version,
    pub status: Option<NetResult<Status>>,
    pub headers: Headers,
    pub body: Option<NetResult<Body>>,
}

impl ResponseBuilder {
    /// Returns a new `ResponseBuilder` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP protocol version.
    #[must_use]
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = version;
        self
    }

    /// Sets the response status using the given status code.
    #[must_use]
    pub fn status_code(&mut self, code: u16) -> &mut Self {
        self.status = Some(Status::try_from(code));
        self
    }

    /// Inserts a header entry from the given name and value.
    #[must_use]
    pub fn header(&mut self, name: &str, value: &[u8]) -> &mut Self {
        self.headers.header(name, value);
        self
    }

    /// Appends the header entries from `other`.
    #[must_use]
    pub fn headers(&mut self, mut other: Headers) -> &mut Self {
        self.headers.append(&mut other);
        self
    }

    /// Sets the response body based on the given `Target`.
    #[must_use]
    pub fn target(&mut self, target: Target) -> &mut Self {
        self.body = Some(Body::try_from(target));
        self
    }

    /// Sets the response body.
    #[must_use]
    pub fn body(&mut self, body: Body) -> &mut Self {
        self.body = Some(Ok(body));
        self
    }

    /// Builds and returns a new `Response` instance.
    ///
    /// # Errors
    /// 
    /// Returns an error if an invalid status code was set or if an error
    /// occurred while converting from a route `Target` to a response `Body`.
    pub fn build(&mut self) -> NetResult<Response> {
        let status = match self.status.take() {
            Some(Err(e)) => Err(e)?,
            Some(Ok(status)) => status,
            None => Status::default(),
        };

        let body = match self.body.take() {
            Some(Err(e)) => Err(e)?,
            Some(Ok(body)) => body,
            None => Body::default(),
        };

        // Ensure the default response headers are set.
        self.headers.default_response_headers(&body);

        Ok(Response {
            version: self.version,
            status,
            headers: self.headers.clone(),
            body
        })
    }
}

/// An HTTP response.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Response {
    pub version: Version,
    pub status: Status,
    pub headers: Headers,
    pub body: Body,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{} {}", &self.version, &self.status)?;

        writeln!(f, "{}", &self.headers)?;

        if self.body.is_printable() {
            writeln!(f, "{}", &self.body)?;
        }

        Ok(())
    }
}

impl FromStr for Response {
    type Err = NetError;

    fn from_str(res: &str) -> NetResult<Self> {
        Self::try_from(res.as_bytes())
    }
}

impl TryFrom<&[u8]> for Response {
    type Error = NetError;

    fn try_from(input: &[u8]) -> NetResult<Self> {
        // Expect HTTP responses to start with 'H' (e.g. "HTTP/1.1").
        let start = input
            .iter()
            .position(|&b| b == b'H')
            .ok_or(NetError::BadResponse)?;

        let mut lines = input[start..].split_inclusive(|&b| b == b'\n');

        let first_line = lines.next().ok_or(NetError::BadResponse)?;

        let mut tokens = first_line.splitn(2, |&b| b == b' ');

        let version = Version::try_from(tokens.next())?;
        let status = Status::try_from(tokens.next())?;

        let headers = lines
            .by_ref()
            .map(utils::trim)
            .take_while(|line| !line.is_empty())
            .map(Header::try_from)
            .collect::<NetResult<Headers>>()?;

        let body = lines
            .flatten()
            .copied()
            .collect::<Vec<u8>>();

        let body = headers
            .get(&CONTENT_TYPE)
            .map_or(
                Body::Empty,
                |content_type| {
                    let content_type = content_type.as_str();
                    Body::from_content_type(&body, &content_type)
                });

        Ok(Self { version, status, headers, body })
    }
}

impl Response {
    /// Returns a new `ResponseBuilder` instance.
    #[must_use]
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    /// Returns a default `Response` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Returns a reference to the response `Status`.
    #[must_use]
    pub const fn status(&self) -> &Status {
        &self.status
    }

    /// Returns the status line as a `String` with plain formatting.
    #[must_use]
    pub fn status_line_to_plain_string(&self) -> String {
        format!("{} {}", &self.version, &self.status)
    }

    /// Returns the status line as a `String` with color formatting.
    #[must_use]
    pub fn status_line_to_color_string(&self) -> String {
        format!("{MAGENTA}{} {}{RESET}", &self.version, &self.status)
    }

    /// Returns a reference to the response `Headers`.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns a reference to the response `Body`.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }
}
