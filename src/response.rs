use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::iter;
use std::str::{self, FromStr};

use crate::{
    Body, HeaderName, HeaderValue, Headers, NetParseError, NetResult, Status,
    Target, Version,
};
use crate::headers::names::CONTENT_TYPE;
use crate::style::colors::{MAGENTA, RESET};
use crate::utils;

/// An HTTP response builder object.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResponseBuilder {
    pub version: Option<Version>,
    pub status: Option<Result<Status, NetParseError>>,
    pub headers: Option<Headers>,
    pub body: Option<Result<Body, NetParseError>>,
}

impl ResponseBuilder {
    /// Returns a new `ResponseBuilder` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the response status using the given status `code`.
    pub fn status_code(&mut self, code: u16) -> &mut Self {
        let status = Status::try_from(code);
        self.status = Some(status);
        self
    }

    /// Sets the response status.
    pub fn status(&mut self, status: Status) -> &mut Self {
        self.status = Some(Ok(status));
        self
    }

    /// Sets the protocol version.
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    /// Inserts a response header.
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

    /// Sets the response headers.
    pub fn headers(&mut self, mut headers: Headers) -> &mut Self {
        match self.headers.as_mut() {
            Some(hdrs) => hdrs.append(&mut headers),
            None => self.headers = Some(headers),
        }

        self
    }

    /// Sets the response body based on the given `Target`.
    pub fn target(&mut self, target: Target) -> &mut Self {
        if !target.is_empty() {
            self.body = Some(Body::try_from(target));
        }

        self
    }

    /// Sets the response body.
    pub fn body(&mut self, body: Body) -> &mut Self {
        if !body.is_empty() {
            self.body = Some(Ok(body));
        }

        self
    }

    /// Builds and returns a new `Response`.
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

        let mut res = Response {
            version: self.version.take().unwrap_or_default(),
            status,
            headers: self.headers.take().unwrap_or_default(),
            body
        };

        // Ensure the default response headers are set.
        res.headers.default_response_headers(&res.body);
        Ok(res)
    }
}

/// Contains the components of an HTTP response.
#[derive(Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Response {
    pub version: Version,
    pub status: Status,
    pub headers: Headers,
    pub body: Body,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{} {}", self.version, self.status)?;

        for (name, value) in &self.headers.0 {
            writeln!(f, "{name}: {value}")?;
        }

        if self.body.is_printable() {
            writeln!(f, "{}", &self.body)?;
        }

        Ok(())
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "Response {{")?;
        writeln!(f, "    version: {:?},", &self.version)?;
        writeln!(f, "    status: {:?},", &self.status)?;
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

impl FromStr for Response {
    type Err = NetParseError;

    fn from_str(res: &str) -> Result<Self, Self::Err> {
        Self::try_from(res.as_bytes())
    }
}

impl TryFrom<&[u8]> for Response {
    type Error = NetParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let start = bytes
            .iter()
            .position(|&b| b == b'H')
            .ok_or(NetParseError::StatusLine)?;

        let mut lines = bytes[start..].split(|b| *b == b'\n');

        // Parse the status line.
        let (version, status) = lines
            .next()
            .ok_or(NetParseError::StatusLine)
            .and_then(Self::parse_status_line)?;

        let mut headers = Headers::new();

        // Collect the trimmed header lines into a new iterator.
        let header_lines = lines
            .by_ref()
            .map_while(|line| {
                let line = utils::trim(line);

                if line.is_empty() {
                    None
                } else {
                    Some(line)
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
            .map_or(Cow::Borrowed(""), |value| value.as_str());

        let body = Body::from_content_type(&body_vec, &content_type);

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

    /// Returns a reference to the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// Returns a reference to the `Status` for this `Response`.
    #[must_use]
    pub const fn status(&self) -> &Status {
        &self.status
    }

    /// Returns the status line as a `String` with plain formatting.
    #[must_use]
    pub fn status_line_to_plain_string(&self) -> String {
        format!("{} {}", self.version, self.status)
    }

    /// Returns the status line as a `String` with color formatting.
    #[must_use]
    pub fn status_line_to_color_string(&self) -> String {
        format!("{MAGENTA}{} {}{RESET}", self.version, self.status)
    }

    /// Parses a bytes slice into a `Version` and a `Status`.
    ///
    /// # Errors
    /// 
    /// Returns an error if status line parsing fails.
    pub fn parse_status_line(
        line: &[u8]
    ) -> Result<(Version, Status), NetParseError> {
        let mut parts = utils::trim(line).splitn(2, |&b| b == b' ');

        let (Some(version), Some(status)) = (parts.next(), parts.next()) else {
            return Err(NetParseError::StatusLine);
        };

        let version = utils::trim_end(version);
        let version = Version::try_from(version)?;

        let status = utils::trim_start(status);
        let status = Status::try_from(status)?;

        Ok((version, status))
    }

    /// Returns a reference to the headers for this `Response`.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the header represented by the given `HeaderName` key
    /// is present.
    #[must_use]
    pub fn contains(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Inserts a header into the `Response`.
    pub fn header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    /// Returns a reference to the message `Body`.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }
}
