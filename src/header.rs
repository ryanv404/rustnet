use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::SocketAddr;
use std::str::FromStr;

use crate::{NetError, NetParseError, NetResult};

pub mod names;
pub mod values;

pub use names::{header_consts::*, HeaderKind, HeaderName};
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

    fn from_str(line: &str) -> NetResult<Self> {
        line.trim()
            .split_once(':')
            .ok_or(NetError::Parse(NetParseError::Header))
            .map(|(name, value)| Self::new(name, value))
    }
}

impl Header {
    /// Returns a new `Header` from the provided name and value.
    #[must_use]
    pub fn new(name: &str, value: &str) -> Self {
        let name = HeaderName::from(name);
        let value = HeaderValue::from(value);
        Self { name, value }
    }
}

/// A wrapper around an object that maps header names to header values.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Headers(pub BTreeMap<HeaderName, HeaderValue>);

impl FromStr for Headers {
    type Err = NetError;

    fn from_str(headers_str: &str) -> NetResult<Self> {
        let mut num_headers = 0;
        let mut headers = Self::new();

        let lines = headers_str
            .trim_start()
            .split('\n');

        for line in lines {
            num_headers += 1;

            if num_headers >= MAX_HEADERS {
                return Err(NetParseError::TooManyHeaders)?;
            }

            let trimmed_line = line.trim();

            // Check for end of headers section.
            if trimmed_line.is_empty() {
                break;
            }

            trimmed_line
                .parse::<Header>()
                .map(|hdr| headers.insert(hdr.name, hdr.value))?;
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

    /// Inserts a collection of default request headers.
    pub fn default_request_headers(&mut self) {
        todo!();
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

    // Common logic for the to_plain_string and to_color_string functions.
    fn string_helper(&self, use_color: bool) -> String {
        const BLU: &str = "\x1b[94m";
        const YLW: &str = "\x1b[96m";
        const CLR: &str = "\x1b[0m";

        let mut buf = String::new();

        if !self.is_empty() {
            self.0.iter().for_each(|(name, value)| {
                let header = if use_color {
                    format!("{BLU}{name}{CLR}: {YLW}{value}{CLR}\n")
                } else {
                    format!("{name}: {value}\n")
                };

                buf.push_str(&header);
            });
        }

        buf
    }

    /// Returns the headers as a `String` with plain formatting.
    #[must_use]
    pub fn to_plain_string(&self) -> String {
        self.string_helper(false)
    }

    /// Returns the headers as a `String` with color formatting.
    #[must_use]
    pub fn to_color_string(&self) -> String {
        self.string_helper(true)
    }
}
