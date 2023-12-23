use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufWriter, StdoutLock, Write};
use std::net::SocketAddr;
use std::str::FromStr;

use crate::{Body, NetError, NetParseError, NetResult};

pub mod names;
pub mod values;

pub use names::{HeaderName, HeaderNameInner};
pub use names::header_name::{
    self, ACCEPT, ACCEPT_ENCODING, CACHE_CONTROL, CONNECTION, CONTENT_LENGTH,
    CONTENT_TYPE, HOST, SERVER, USER_AGENT,
};
pub use values::HeaderValue;

pub const MAX_HEADERS: u16 = 1024;

/// Represents a single header field line.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Header {
    pub name: HeaderName,
    pub value: HeaderValue,
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}: {}", self.name, self.value)
    }
}

impl FromStr for Header {
    type Err = NetError;

    fn from_str(header: &str) -> NetResult<Self> {
        header.trim()
            .split_once(':')
            .ok_or(NetParseError::Header.into())
            .map(Into::into)
    }
}

impl From<(&str, &str)> for Header {
    fn from((name, value): (&str, &str)) -> Self {
        Self { name: name.into(), value: value.into() }
    }
}

/// A wrapper around an object that maps header names to header values.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Headers(pub BTreeMap<HeaderName, HeaderValue>);

impl FromStr for Headers {
    type Err = NetError;

    fn from_str(many_headers: &str) -> NetResult<Self> {
        let mut headers = Self::new();

        let mut lines = many_headers.trim().lines();

        while let Some(line) = lines.next() {
            headers.parse_and_insert_header(line)?;
        }

        Ok(headers)
    }
}

impl Headers {
    /// Returns a new `Headers` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a the value associated with the given `HeaderName`, if present.
    #[must_use]
    pub fn get(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.0.get(name)
    }

    /// Returns true if there are no header entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns true if the header represented by `HeaderName` is present.
    #[must_use]
    pub fn contains(&self, name: &HeaderName) -> bool {
        self.0.contains_key(name)
    }

    /// Removes and returns the first entry in the map.
    #[must_use]
    pub fn pop_first(&mut self) -> Option<(HeaderName, HeaderValue)> {
        self.0.pop_first()
    }

    /// Returns the entry for associated with the given `HeaderName` key.
    #[must_use]
    pub fn entry(
        &mut self,
        name: HeaderName,
    ) -> Entry<HeaderName, HeaderValue> {
        self.0.entry(name)
    }

    /// Inserts a header field entry.
    pub fn insert(&mut self, name: HeaderName, value: HeaderValue) {
        self.entry(name)
            .and_modify(|val| *val = value.clone())
            .or_insert(value);
    }

    /// Inserts a header if one with the same `HeaderName` is not already
    /// present.
    pub fn insert_if_empty(&mut self, name: HeaderName, value: HeaderValue) {
        self.entry(name).or_insert(value);
    }

    /// Parses a `Header` from the given string slice and inserts the
    /// resulting header into this `Headers` map.
    ///
    /// # Errors
    ///
    /// Returns an error if `Header::from_str` returns an error.
    pub fn parse_and_insert_header(&mut self, line: &str) -> NetResult<()> {
        let header = Header::from_str(line)?;
        self.insert(header.name.clone(), header.value);
        Ok(())
    }

    /// Removes a header field entry.
    pub fn remove(&mut self, name: &HeaderName) {
        self.0.remove(name);
    }

    /// Returns the number of header field entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Clears all header field entries.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Inserts sensible values for a default set of request headers if they
    /// are not already present.
    pub fn default_request_headers(&mut self, body: &Body, addr: &SocketAddr) {
        self.insert_if_empty(ACCEPT, "*/*".into());
        self.insert_if_empty(HOST, addr.into());
        self.insert_if_empty(USER_AGENT, "rustnet/0.1".into());
        self.insert_if_empty(CONTENT_LENGTH, body.len().into());

        if let Some(con_type) = body.as_content_type() {
            self.insert_if_empty(CONTENT_TYPE, con_type.into());
        }
    }

    /// Inserts a collection of default server response headers.
    pub fn default_response_headers(&mut self) {
        todo!();
    }

    /// Inserts a new header with the given name and value.
    pub fn header(&mut self, name: &str, value: &str) {
        self.insert(HeaderName::from(name), HeaderValue::from(value));
    }

    /// Inserts a Host header that is parsed from the given `SocketAddr`.
    pub fn host(&mut self, host: &SocketAddr) {
        let ip = host.ip();
        let port = host.port();
        self.insert(HOST, format!("{ip}:{port}").into());
    }

    /// Inserts the default User-Agent header.
    pub fn user_agent(&mut self, agent: &str) {
        self.insert(USER_AGENT, agent.into());
    }

    /// Inserts an Accept header with the given value.
    pub fn accept(&mut self, accepted: &str) {
        self.insert(ACCEPT, accepted.into());
    }

    /// Inserts an Accept-Encoding header with the given value.
    pub fn accept_encoding(&mut self, encoding: &str) {
        self.insert(ACCEPT_ENCODING, encoding.into());
    }

    /// Inserts a Server header with the given value.
    pub fn server(&mut self, server: &str) {
        self.insert(SERVER, server.into());
    }

    /// Inserts a Connection header with the given value.
    pub fn connection(&mut self, conn: &str) {
        self.insert(CONNECTION, conn.into());
    }

    /// Inserts a Content-Length header with the given value.
    pub fn content_length(&mut self, len: usize) {
        self.insert(CONTENT_LENGTH, len.into());
    }

    /// Inserts a Content-Type header with the given value.
    pub fn content_type(&mut self, content_type: &str) {
        self.insert(CONTENT_TYPE, content_type.into());
    }

    /// Inserts a Cache-Control header with the given value.
    pub fn cache_control(&mut self, directive: &str) {
        self.insert(CACHE_CONTROL, directive.into());
    }

    /// Writes the headers to a `BufWriter` with plain formatting.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the provided `BufWriter` fails.
    pub fn write_plain(
        &self,
        writer: &mut BufWriter<StdoutLock<'_>>
    ) -> NetResult<()> {
        for (name, value) in &self.0 {
            writeln!(writer, "{name}: {value}")?;
        }

        Ok(())
    }

    /// Writes the headers to a `BufWriter` with color formatting.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the provided `BufWriter` fails.
    pub fn write_color(
        &self,
        writer: &mut BufWriter<StdoutLock<'_>>
    ) -> NetResult<()> {
        use crate::colors::{BLU, CLR, CYAN};

        for (name, value) in &self.0 {
            writeln!(writer, "{BLU}{name}{CLR}: {CYAN}{value}{CLR}")?;
        }

        Ok(())
    }

    /// Returns the Content-Length and Content-Type header values from this
    /// `Headers` map.
    pub fn get_content_len_and_type(&self) -> NetResult<(u64, String)> {
        let cl_value = self.get(&CONTENT_LENGTH);
        let ct_value = self.get(&CONTENT_TYPE);

        if cl_value.is_none() || ct_value.is_none() {
            return Ok((0, String::new()));
        }

        let body_len = cl_value
            .ok_or(NetParseError::Body.into())
            .map(ToString::to_string)
            .and_then(|len| len.trim().parse::<usize>()
                .map_err(|_| NetParseError::Body))?;

        if body_len == 0 {
            return Ok((0, String::new()));
        }

        let content_len = u64::try_from(body_len)
            .map_err(|_| NetParseError::Body)?;

        let content_type = ct_value
            .map(ToString::to_string)
            .ok_or(NetParseError::Body)?;

        if content_type.is_empty() {
            // Return error since content length is greater than zero.
            return Err(NetParseError::Body)?;
        }

        Ok((content_len, content_type))
    }
}
