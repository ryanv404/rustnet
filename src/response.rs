use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufWriter, StdoutLock, Write};
use std::str::FromStr;

use crate::{
    Body, Connection, HeaderName, HeaderValue, Headers, Method, NetError,
    NetParseError, NetResult, Status, StatusCode, Target, Version,
};
use crate::colors::{CLR, PURP};
use crate::header_name::CONNECTION;

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

        let mut tokens = (&line[start..]).splitn(2, |b| *b == b' ');
        let token1 = tokens.next();
        let token2 = tokens.next();

        match (token1, token2) {
            (Some(token1), Some(token2)) => {
                let version = Version::try_from(token1)?;
                let status = Status::try_from(token2)?;
                Ok(Self { version, status })
            },
            (_, _) => Err(NetParseError::StatusLine)?,
        }
    }
}

impl FromStr for StatusLine {
    type Err = NetError;

    fn from_str(line: &str) -> NetResult<Self> {
        let start = line
            .find("HTTP")
            .ok_or(NetParseError::StatusLine)?;

        (&line[start..])
            .trim()
            .split_once(' ')
            .ok_or(NetParseError::StatusLine.into())
            .and_then(|(token1, token2)| {
                let version = Version::from_str(token1)?;
                let status = Status::from_str(token2)?;
                Ok(Self { version, status })
            })
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
    /// Writes the `StatusLine` to a `BufWriter` with plain formatting.
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

    /// Writes the `StatusLine` to a `BufWriter` with color formatting.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the provided `BufWriter` fails.
    pub fn write_color(
        &self,
        writer: &mut BufWriter<StdoutLock<'_>>
    ) -> NetResult<()> {
        writeln!(writer, "{PURP}{self}{CLR}")?;
        Ok(())
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
            let body = String::from_utf8_lossy(self.body.as_bytes());
            writeln!(f, "{body}")?;
        }

        Ok(())
    }
}

impl TryFrom<u16> for Response {
    type Error = NetError;

    fn try_from(code: u16) -> NetResult<Self> {
        let status_line = StatusLine::try_from(code)?;
        let headers = Headers::new();
        let body = Body::Empty;

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
    pub fn status_msg(&self) -> &str {
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

    /// Writes an HTTP response to a `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::send_response` fails.
    pub fn send(&mut self, conn: &mut Connection) -> NetResult<()> {
        conn.send_response(self)
    }

    /// Reads and parses an HTTP response from a `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::recv_response` fails.
    pub fn recv(conn: &mut Connection) -> NetResult<Self> {
        conn.recv_response()
    }
}
