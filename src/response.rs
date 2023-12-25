use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::iter;
use std::str::FromStr;

use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetError,
    NetParseError, NetResult, Status, StatusCode, Target, Version,
};
use crate::colors::{CLR, PURP};
use crate::header_name::{CONNECTION, CONTENT_TYPE};
use crate::util;

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

impl TryFrom<&[u8]> for StatusLine {
    type Error = NetError;

    fn try_from(line: &[u8]) -> NetResult<Self> {
        let start = line
            .iter()
            .position(|b| *b == b'H')
            .ok_or(NetParseError::StatusLine)?;

        let mut tokens = line[start..].splitn(2, |b| *b == b' ');

        let version = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::StatusLine))
            .and_then(Version::try_from)?;

        let status = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::StatusLine))
            .and_then(Status::try_from)?;

        Ok(Self { version, status })
    }
}

impl FromStr for StatusLine {
    type Err = NetError;

    fn from_str(line: &str) -> NetResult<Self> {
        Self::try_from(line.as_bytes())
    }
}

impl TryFrom<u16> for StatusLine {
    type Error = NetError;

    fn try_from(code: u16) -> NetResult<Self> {
        StatusCode::try_from(code).map(Into::into)
    }
}

impl From<StatusCode> for StatusLine {
    fn from(status_code: StatusCode) -> Self {
        Self {
            version: Version::OneDotOne,
            status: Status(status_code)
        }
    }
}

impl StatusLine {
    /// Returns the `StatusLine` as a `String` with color formatting.
    #[must_use]
    pub fn to_color_string(&self) -> String {
        format!("{PURP}{self}{CLR}")
    }
}

/// Contains the components of an HTTP response.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
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

impl TryFrom<u16> for Response {
    type Error = NetError;

    fn try_from(code: u16) -> NetResult<Self> {
        let res = Self {
            status_line: StatusLine::try_from(code)?,
            headers: Headers::new(),
            body: Body::Empty
        };

        Ok(res)
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

    fn try_from(bytes: &[u8]) -> NetResult<Self> {
        let trimmed = util::trim_start_bytes(bytes);

        let mut lines = trimmed.split(|b| *b == b'\n');

        // Parse the StatusLine.
        let status_line = lines
            .next()
            .ok_or(NetError::Parse(NetParseError::StatusLine))
            .and_then(StatusLine::try_from)?;

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

        Ok(Self { status_line, headers, body })
    }
}

impl Response {
    /// Constructs a new `Response` based on a `Target` and a `Route`.
    ///
    /// # Errors
    ///
    /// Returns an error if the `Response` could not be constructed.
    pub fn from_target(target: Target, status: u16) -> NetResult<Self> {
        let mut res = Self::try_from(status)?;

        res.body = Body::try_from(target)?;

        // Set the Cache-Control header.
        if res.body.is_favicon() {
            res.headers.cache_control("max-age=604800");
        } else {
            res.headers.cache_control("no-cache");
        }

        // Set the Content-Type header.
        if let Some(cont_type) = res.body.as_content_type() {
            res.headers.content_type(cont_type);
        }

        // Set the Content-Length header.
        if !res.body.is_empty() {
            res.headers.content_length(res.body.len());
        }

        Ok(res)
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

    /// Returns true if the Connection header's value is "close".
    #[must_use]
    pub fn has_close_connection_header(&self) -> bool {
        self.headers.get(&CONNECTION) == Some(&HeaderValue::from("close"))
    }

    /// Returns true if a body is permitted for this `Response`.
    #[must_use]
    pub fn body_is_permitted(&self, method: &Method) -> bool {
        match self.status_code() {
            // 1xx (Informational), 204 (No Content), and 304 (Not Modified).
            100..=199 | 204 | 304 => false,
            // CONNECT responses with a 2xx (Success) status.
            200..=299 if *method == Method::Connect => false,
            // HEAD responses.
            _ if *method == Method::Head => false,
            _ => true,
        }
    }

    /// Returns a reference to the message `Body`.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }
}
