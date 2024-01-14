use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::SocketAddr;
use std::str::{self, FromStr};

use crate::{
    Body, NetError, NetParseError, NetResult, DEFAULT_NAME,
};
use crate::style::colors::{BLUE, CYAN, RESET};
use crate::utils::{self, Trim};

pub mod names;
pub mod values;

pub use names::HeaderName;
use names::{
    ACCEPT, ACCEPT_ENCODING, CACHE_CONTROL, CONNECTION, CONTENT_LENGTH,
    CONTENT_TYPE, DATE, HOST, SERVER, USER_AGENT,
};
pub use values::HeaderValue;

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

    /// Inserts a new header with the given name and value.
    pub fn header(&mut self, name: &str, value: &str) {
        self.insert(HeaderName::from(name), HeaderValue::from(value));
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

    /// Inserts a sensible set default of request headers.
    pub fn default_request_headers(
        &mut self,
        body: &Body,
        remote_addr: Option<SocketAddr>
    ) {
        if !self.contains(&ACCEPT) {
            self.add_accept("*/*");
        }

        if !self.contains(&CONTENT_LENGTH) && !body.is_empty() {
            self.add_content_length(body.len());
        }

        if !self.contains(&CONTENT_TYPE) && !body.is_empty() {
            if let Some(content_type) = body.as_content_type() {
                self.add_content_type(content_type);
            }
        }

        if !self.contains(&DATE) {
            self.add_date();
        }

        if !self.contains(&HOST) {
            if let Some(addr) = remote_addr {
                self.add_host(addr);
            }
        }

        if !self.contains(&USER_AGENT) {
            self.add_user_agent();
        }
    }

    /// Inserts a sensible set of default response headers.
    pub fn default_response_headers(&mut self, body: &Body) {
        if !self.contains(&CACHE_CONTROL) {
            // Cache favicon for 1 week.
            if body.is_favicon() {
                self.add_cache_control("max-age=604800");
            } else {
                self.add_cache_control("no-cache");
            }
        }

        if !self.contains(&CONTENT_LENGTH) && !body.is_empty() {
            self.add_content_length(body.len());
        }

        if !self.contains(&CONTENT_TYPE) && !body.is_empty() {
            if let Some(content_type) = body.as_content_type() {
                self.add_content_type(content_type);
            }
        }

        if !self.contains(&DATE) {
            self.add_date();
        }

        if !self.contains(&SERVER) {
            self.add_server();
        }
    }

    /// Inserts an Accept header with the given value.
    pub fn add_accept(&mut self, accepted: &str) {
        self.insert(ACCEPT, accepted.into());
    }

    /// Inserts an Accept-Encoding header with the given value.
    pub fn add_accept_encoding(&mut self, encoding: &str) {
        self.insert(ACCEPT_ENCODING, encoding.into());
    }

    /// Inserts a Cache-Control header with the given value.
    pub fn add_cache_control(&mut self, directive: &str) {
        self.insert(CACHE_CONTROL, directive.into());
    }

    /// Inserts a Connection header with the given value.
    pub fn add_connection(&mut self, conn: &str) {
        self.insert(CONNECTION, conn.into());
    }

    /// Inserts a Content-Length header with the given value.
    pub fn add_content_length(&mut self, len: usize) {
        self.insert(CONTENT_LENGTH, len.into());
    }

    /// Inserts a Content-Type header with the given value.
    pub fn add_content_type(&mut self, content_type: &str) {
        self.insert(CONTENT_TYPE, content_type.into());
    }

    /// Inserts a Date header with the current date and time, if possible.
    pub fn add_date(&mut self) {
        if let Some(date_value) = utils::get_datetime() {
            self.insert(DATE, date_value);
        }
    }

    /// Inserts a Host header from the given `SocketAddr`.
    pub fn add_host(&mut self, host: SocketAddr) {
        self.insert(HOST, format!("{host}").into());
    }

    /// Inserts the default Server header.
    pub fn add_server(&mut self) {
        self.insert(SERVER, DEFAULT_NAME.into());
    }

    /// Inserts the default User-Agent header.
    pub fn add_user_agent(&mut self) {
        self.insert(USER_AGENT, DEFAULT_NAME.into());
    }

    /// Returns the `Headers` as a `String` with color formatting.
    #[must_use]
    pub fn to_color_string(&self) -> String {
        let mut headers = String::new();

        for (name, value) in &self.0 {
            let header = format!(
                "{BLUE}{name}{RESET}: {CYAN}{value}{RESET}\n"
            );

            headers.push_str(&header);
        }

        headers
    }
}
