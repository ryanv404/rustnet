use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::SocketAddr;
use std::str::{self, FromStr};

use crate::{Body, NetError, NetParseError, NetResult, DEFAULT_NAME};
use crate::style::colors::{BR_BLU, BR_CYAN, CLR};
use crate::util::Trim;

pub mod names;
pub mod values;

use names::{
    ACCEPT, ACCEPT_ENCODING, CACHE_CONTROL, CONNECTION, CONTENT_LENGTH,
    CONTENT_TYPE, HOST, SERVER, USER_AGENT,
};
pub use names::{HeaderName, HeaderNameInner};
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
        write!(f, "{}: {}", &self.name, &self.value)
    }
}

impl FromStr for Header {
    type Err = NetParseError;

    fn from_str(header: &str) -> Result<Self, Self::Err> {
        Self::try_from(header.as_bytes())
    }
}

impl TryFrom<&[u8]> for Header {
    type Error = NetParseError;

    fn try_from(header: &[u8]) -> Result<Self, Self::Error> {
        let mut tokens = header.splitn(2, |b| *b == b':');

        let name = tokens
            .next()
            .ok_or(NetParseError::Header)
            .and_then(|name| str::from_utf8(name)
                .map_err(|_| NetParseError::Header))
            .map(|name| HeaderName::from(name.trim()))?;

        let value = tokens
            .next()
            .ok_or(NetParseError::Header)
            .map(HeaderValue::from)?;

        Ok(Self { name, value })
    }
}

/// A wrapper around an object that maps header names to header values.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Headers(pub BTreeMap<HeaderName, HeaderValue>);

impl Display for Headers {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for (name, value) in &self.0 {
            write!(f, "{name}: {value}\r\n")?;
        }

        Ok(())
    }
}

impl FromStr for Headers {
    type Err = NetParseError;

    fn from_str(many_headers: &str) -> Result<Self, Self::Err> {
        Self::try_from(many_headers.as_bytes())
    }
}

impl TryFrom<&mut Vec<u8>> for Headers {
    type Error = NetError;

    fn try_from(many_headers: &mut Vec<u8>) -> NetResult<Self> {
        let headers = Self::try_from(&many_headers[..])?;
        Ok(headers)
    }
}

impl TryFrom<&[u8]> for Headers {
    type Error = NetParseError;

    fn try_from(many_headers: &[u8]) -> Result<Self, Self::Error> {
        let mut headers = Self::new();

        let lines = many_headers
            .split(|b| *b == b'\n')
            .map_while(|line| {
                let line = line.trim();

                if line.is_empty() {
                    None
                } else {
                    Some(line)
                }
            });

        for line in lines {
            headers.insert_header_from_bytes(line)?;
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
    pub fn entry(&mut self, name: HeaderName) -> Entry<HeaderName, HeaderValue> {
        self.0.entry(name)
    }

    /// Append another `Headers` collection to this one.
    pub fn append(&mut self, other: &mut Self) {
        self.0.append(&mut other.0);
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
    /// Returns an error if `Header` parsing fails.
    pub fn insert_header_from_str(
        &mut self,
        line: &str
    ) -> Result<(), NetParseError> {
        let header = Header::from_str(line)?;
        self.insert(header.name.clone(), header.value);
        Ok(())
    }

    /// Parses a `Header` from the given bytes slice and inserts the
    /// resulting header into this `Headers` map.
    ///
    /// # Errors
    ///
    /// Returns an error if `Header` parsing fails.
    pub fn insert_header_from_bytes(
        &mut self,
        line: &[u8]
    ) -> Result<(), NetParseError> {
        let header = Header::try_from(line)?;
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
    pub fn default_request_headers(&mut self, body: &Body, addr: SocketAddr) {
        self.insert_if_empty(HOST, addr.into());
        self.insert_if_empty(ACCEPT, "*/*".into());
        self.insert_if_empty(USER_AGENT, DEFAULT_NAME.into());
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
    pub fn host(&mut self, host: SocketAddr) {
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

    /// Returns the `Headers` as a `String` with color formatting.
    #[must_use]
    pub fn to_color_string(&self) -> String {
        let mut headers = String::new();

        for (name, value) in &self.0 {
            let header = format!(
                "{BR_BLU}{name}{CLR}: {BR_CYAN}{value}{CLR}\n"
            );

            headers.push_str(&header);
        }

        headers
    }
}
