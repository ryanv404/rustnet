use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::IpAddr;
use std::str::FromStr;

use crate::{NetError, NetResult, NetParseError};

pub mod names;
pub mod values;

pub use names::{header_consts::*, HeaderKind, HeaderName};
pub use values::HeaderValue;

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
            .and_then(|(name, value)| {
                let name = name.parse::<HeaderName>()?;
                let value = value.parse::<HeaderValue>()?;
                Ok(Self { name, value })
            })
    }
}

/// A wrapper around an object that maps header names to header values.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Headers(pub BTreeMap<HeaderName, HeaderValue>);

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
    pub fn entry(&mut self, name: HeaderName) -> Entry<HeaderName, HeaderValue> {
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
        self.server();
        self.connection("keep-alive");
        self.content_length(0);
    }

    /// Inserts a Host header with the value "ip:port".
    pub fn host(&mut self, ip: IpAddr, port: u16) {
        self.insert(HOST, format!("{ip}:{port}").into());
    }

    /// Inserts the default User-Agent header.
    pub fn user_agent(&mut self) {
        let agent = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
        self.insert(USER_AGENT, agent.as_bytes().into());
    }

    /// Inserts an Accept header with the given value.
    pub fn accept(&mut self, value: &str) {
        self.insert(ACCEPT, value.as_bytes().into());
    }

    /// Inserts the default Server header.
    pub fn server(&mut self) {
        let server = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
        self.insert(SERVER, server.as_bytes().into());
    }

    /// Inserts a Connection header with the given value.
    pub fn connection(&mut self, value: &str) {
        self.insert(CONNECTION, value.as_bytes().into());
    }

    /// Inserts a Content-Length header with the given value.
    pub fn content_length(&mut self, len: usize) {
        self.insert(CONTENT_LENGTH, len.into());
    }

    /// Inserts a Content-Type header with the given value.
    pub fn content_type(&mut self, value: &str) {
        self.insert(CONTENT_TYPE, value.as_bytes().into());
    }

    /// Inserts a Cache-Control header with the given value.
    pub fn cache_control(&mut self, value: &str) {
        self.insert(CACHE_CONTROL, value.as_bytes().into());
    }
}
