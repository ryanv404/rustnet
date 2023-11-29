use std::collections::{btree_map::Entry, BTreeMap};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::IpAddr;

use crate::NetResult;
use crate::consts::{
    ACCEPT, CACHE_CONTROL, CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, HOST,
    LOCATION, SERVER, USER_AGENT, WWW_AUTHENTICATE, X_MORE_INFO,
};

pub mod names;
pub mod values;

pub use names::{header_consts, HeaderKind, HeaderName};
pub use values::HeaderValue;

/// Represents a single header field line.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Header {
    pub name: HeaderName,
    pub value: HeaderValue,
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}: {}", self.name, self.value)
    }
}

impl Header {
    /// Parses a string slice into a `Header`.
    pub fn parse(line: &str) -> NetResult<Header> {
        let mut tokens = line.splitn(2, ':');
        let name = HeaderName::parse(tokens.next())?;
        let value = HeaderValue::parse(tokens.next())?;
        Ok(Self { name, value })
    }
}

/// A wrapper around an object that maps header names to header values.
#[derive(Clone, Debug, Hash, PartialOrd, Ord)]
pub struct Headers(pub BTreeMap<HeaderName, HeaderValue>);

impl Default for Headers {
    fn default() -> Self {
        Self(BTreeMap::<HeaderName, HeaderValue>::new())
    }
}

impl PartialEq for Headers {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Headers {}

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

    /// Inserts the default response headers.
    pub fn default_response_headers(&mut self) {
        self.insert_server();
        self.insert_connection("keep-alive");
        self.insert_content_length(0);
    }

    /// Inserts a Host header with the value "ip:port".
    pub fn insert_host(&mut self, ip: IpAddr, port: u16) {
        self.insert(HOST, format!("{ip}:{port}").into());
    }

    /// Inserts the default User-Agent header.
    pub fn insert_user_agent(&mut self) {
        let agent = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
        self.insert(USER_AGENT, agent.as_bytes().into());
    }

    /// Inserts an Accept header with the given value.
    pub fn insert_accept(&mut self, value: &str) {
        self.insert(ACCEPT, value.as_bytes().into());
    }

    /// Inserts the default Server header.
    pub fn insert_server(&mut self) {
        let server = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
        self.insert(SERVER, server.as_bytes().into());
    }

    /// Inserts a Connection header with a value of "keep-alive".
    pub fn insert_connection(&mut self, value: &str) {
        self.insert(CONNECTION, value.as_bytes().into());
    }

    /// Inserts a Content-Length header with the given value.
    pub fn insert_content_length(&mut self, len: usize) {
        self.insert(CONTENT_LENGTH, len.into());
    }

    /// Inserts a Content-Type header with the given value.
    pub fn insert_content_type(&mut self, value: &str) {
        self.insert(CONTENT_TYPE, value.as_bytes().into());
    }

    /// Inserts a Cache-Control header with the given value.
    pub fn insert_cache_control(&mut self, value: &str) {
        self.insert(CACHE_CONTROL, value.as_bytes().into());
    }

    /// Updates headers to reflect the httpbin.org style by status code.
    pub fn update_headers_by_status_code(&mut self, code: u16) {
        match code {
            101 => {
                self.remove(&CONTENT_LENGTH);
                self.entry(CONNECTION)
                    .and_modify(|val| *val = b"upgrade"[..].into());
            },
            100 | 102 | 103 | 204 => {
                self.remove(&CONTENT_LENGTH);
            },
            301 | 302 | 303 | 305 | 307 => {
                self.remove(&CONTENT_TYPE);
                self.insert(LOCATION, b"/redirect/1"[..].into());
            },
            304 => {
                self.remove(&CONTENT_TYPE);
                self.remove(&CONTENT_LENGTH);
            },
            401 => {
                self.remove(&CONTENT_TYPE);
                self.insert(WWW_AUTHENTICATE,
                    br#"Basic realm="Fake Realm""#[..].into());
            },
            402 => {
                self.remove(&CONTENT_TYPE);
                self.insert(X_MORE_INFO,
                    b"http://vimeo.com/22053820"[..].into());
            },
            407 | 412 => {
                self.remove(&CONTENT_TYPE);
            },
            418 => {
                self.remove(&CONTENT_TYPE);
                self.insert(X_MORE_INFO,
                    b"http://tools.ietf.org/html/rfc2324"[..].into());
            },
            _ => {},
        }
    }
}

