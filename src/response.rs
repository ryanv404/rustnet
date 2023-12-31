use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::iter;
use std::str::{self, FromStr};

use crate::{
    Body, HeaderName, HeaderValue, Headers, NetParseError, NetResult, Status,
    Target, Version, DEFAULT_NAME,
};
use crate::header::names::CONTENT_TYPE;
use crate::style::colors::{BR_PURP, CLR};
use crate::util;

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
        let version = self.version.take().unwrap_or_default();
        let headers = self.headers.take().unwrap_or_default();

        let status_line = match self.status.take() {
            Some(Err(e)) => Err(e)?,
            Some(Ok(status)) => StatusLine { version, status },
            None => StatusLine { version, status: Status::default() },
        };

        let mut res = match self.body.take() {
            Some(Err(e)) => Err(e)?,
            Some(Ok(body)) => Response { status_line, headers, body },
            None => Response { status_line, headers, body: Body::default() },
        };

        // Ensure standard response headers are set.
        res.headers.server(DEFAULT_NAME);
        res.headers.cache_control("no-cache");

        if !res.body.is_empty() {
            // Cache favicon for 1 week.
            if res.body.is_favicon() {
                res.headers.cache_control("max-age=604800");
            }

            // Ensure the Content-Length is set
            res.headers.content_length(res.body.len());

            // Ensure the Content-Type is set
            if let Some(content_type) = res.body.as_content_type() {
                res.headers.content_type(content_type);
            }
        }

        Ok(res)
    }
}

/// Contains the components of an HTTP status line.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusLine {
    pub version: Version,
    pub status: Status,
}

impl Display for StatusLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {}", self.version, self.status)
    }
}

impl From<Status> for StatusLine {
    fn from(status: Status) -> Self {
        Self {
            version: Version::default(),
            status
        }
    }
}

impl FromStr for StatusLine {
    type Err = NetParseError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let Some(start_idx) = line.find("HTTP") else {
            return Err(NetParseError::StatusLine);
        };

        line[start_idx..]
            .split_once(' ')
            .ok_or(NetParseError::StatusLine)
            .and_then(|(version, status)| {
                let version = Version::from_str(version.trim())?;
                let status = Status::from_str(status.trim())?;

                Ok(Self { version, status })
            })
    }
}

impl TryFrom<&[u8]> for StatusLine {
    type Error = NetParseError;

    fn try_from(line: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(line)
            .map_err(|_| NetParseError::StatusLine)
            .and_then(Self::from_str)
    }
}

impl TryFrom<u16> for StatusLine {
    type Error = NetParseError;

    fn try_from(code: u16) -> Result<Self, Self::Error> {
        Status::try_from(code).map(Into::into)
    }
}

impl StatusLine {
    /// Returns a default `StatusLine` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the `StatusLine` as a `String` with color formatting.
    #[must_use]
    pub fn to_color_string(&self) -> String {
        format!("{BR_PURP}{self}{CLR}")
    }
}

/// Contains the components of an HTTP response.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Response {
    pub status_line: StatusLine,
    pub headers: Headers,
    pub body: Body,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{}", &self.status_line)?;

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
        writeln!(f, "    status_line: StatusLine {{")?;
        write!(f, "        ")?;
        writeln!(f, "version: {:?},", &self.status_line.version)?;
        write!(f, "        ")?;
        writeln!(f, "status: {:?}", &self.status_line.status)?;
        writeln!(f, "    }},")?;
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
        let trimmed = util::trim_start(bytes);

        let mut lines = trimmed.split(|b| *b == b'\n');

        // Parse the StatusLine.
        let status_line = lines
            .next()
            .ok_or(NetParseError::StatusLine)
            .and_then(|line| {
                str::from_utf8(line)
                    .map_err(|_| NetParseError::StatusLine)
                    .and_then(StatusLine::from_str)
            })?;

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

        Ok(Self { status_line, headers, body })
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

    /// Returns a reference to the `StatusLine`.
    #[must_use]
    pub const fn status_line(&self) -> &StatusLine {
        &self.status_line
    }

    /// Returns a reference to the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.status_line.version
    }

    /// Returns a reference to the `Status` for this `Response`.
    #[must_use]
    pub const fn status(&self) -> &Status {
        &self.status_line.status
    }

    /// Returns the `Status` code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_line.status.code()
    }

    /// Returns the reason phrase for the response `Status`.
    #[must_use]
    pub fn status_msg(&self) -> Cow<'_, str> {
        self.status_line.status.msg()
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
