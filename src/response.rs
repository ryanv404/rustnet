use std::borrow::Cow;
use std::collections::VecDeque;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::iter;
use std::str::{self, FromStr};

use crate::{
    Body, Headers, HeaderName, HeaderValue, NetParseError, NetResult, Status,
    Target, Version,
};
use crate::headers::names::CONTENT_TYPE;
use crate::style::colors::{MAGENTA, RESET};
use crate::utils;

/// An HTTP response builder object.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResponseBuilder {
    pub version: Version,
    pub status: Option<Result<Status, NetParseError>>,
    pub headers: Headers,
    pub body: Option<Result<Body, NetParseError>>,
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
#[derive(Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

    fn try_from(input: &[u8]) -> Result<Self, Self::Error> {
        // Expect HTTP responses to start with the ASCII character 'H'.
        let res_start = input
            .iter()
            .position(|&b| b == b'H')
            .ok_or(NetParseError::StatusLine)?;

        let mut lines = utils::trim_start(&input[res_start..])
            .split(|b| *b == b'\n')
            .collect::<VecDeque<&[u8]>>();

        if lines.is_empty() {
            return Err(NetParseError::StatusLine);
        }

        let (version, status) = lines
            .pop_front()
            .ok_or(NetParseError::StatusLine)
            .and_then(Self::parse_status_line)?;

        let mut headers = Headers::new();

        let mut lines_iter = lines.iter();

        while let Some(line) = lines_iter.next() {
            let line = utils::trim(line);

            if line.is_empty() {
                break;
            }

            let mut parts = line.splitn(2, |&b| b == b':');

            let (name, value) = parts
                .next()
                .map(utils::trim_end)
                .ok_or(NetParseError::Header)
                .and_then(HeaderName::try_from)
                .and_then(|name| {
                    let value = parts
                        .next()
                        .map(utils::trim_start)
                        .ok_or(NetParseError::Header)
                        .map(HeaderValue::from)?;

                    Ok((name, value))
                })?;

            headers.insert(name, value);
        }

        let body_bytes = lines_iter
            // Restore newline characters removed by `split` above.
            .flat_map(|line| line.iter().copied().chain(iter::once(b'\n')))
            .collect::<Vec<u8>>();

        let content_type = headers
            .get(&CONTENT_TYPE)
            .map_or(Cow::Borrowed(""), |value| value.as_str());

        let body = Body::from_content_type(&body_bytes, &content_type);

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

    /// Parses a bytes slice into a `Version` and a `Status`.
    ///
    /// # Errors
    /// 
    /// Returns an error if status line parsing fails.
    pub fn parse_status_line(
        line: &[u8]
    ) -> Result<(Version, Status), NetParseError> {
        let mut parts = utils::trim_start(line).split(|&b| b == b' ');

        let version = parts
            .next()
            .map(utils::trim_end)
            .ok_or(NetParseError::StatusLine)
            .and_then(Version::try_from)?;

        let status = parts
            .next()
            .map(utils::trim)
            .ok_or(NetParseError::StatusLine)
            .and_then(Status::try_from)?;

        Ok((version, status))
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
