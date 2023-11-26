use std::collections::BTreeMap;
use std::net::IpAddr;

use crate::NetResult;
use crate::consts::{ACCEPT, HOST, SERVER, USER_AGENT};

pub mod names;
pub mod values;

pub use names::{header_consts, HeaderKind, HeaderName};
pub use values::HeaderValue;

/// A unit struct that contains a header parsing method.
pub struct Header;

impl Header {
    /// Parses a string slice into a `HeaderName` and a `HeaderValue`.
    pub fn parse(line: &str) -> NetResult<(HeaderName, HeaderValue)> {
        let mut tokens = line.splitn(2, ':');

        let hdr_name = HeaderName::parse(tokens.next())?;
        let hdr_value = HeaderValue::parse(tokens.next())?;

        Ok((hdr_name, hdr_value))
    }
}

/// A wrapper around an object that maps header names to header values.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

    pub fn insert(&mut self, name: HeaderName, value: HeaderValue) {
        self.0.entry(name)
            .and_modify(|v| *v = value.clone())
            .or_insert(value);
    }

    pub fn remove(&mut self, name: &HeaderName) {
        self.0.remove(name);
    }

    /// Inserts a Host header with the value "ip:port".
    pub fn insert_host(&mut self, ip: IpAddr, port: u16) {
        let host = format!("{ip}:{port}");
        self.insert(HOST, host.into())
    }

    /// Inserts the default User-Agent header.
    pub fn insert_user_agent(&mut self) {
        let agent = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
        self.insert(USER_AGENT, Vec::from(agent).into())
    }

    /// Inserts an Accept header with a value of "*/*".
    pub fn insert_accept_all(&mut self) {
        self.insert(ACCEPT, Vec::from("*/*").into())
    }

    /// Inserts the default Server header.
    pub fn insert_server(&mut self) {
        let server = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
        self.insert(SERVER, Vec::from(server).into())
    }
}
