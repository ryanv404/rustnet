use std::collections::BTreeMap;
use std::net::IpAddr;

use crate::{NetResult, ParseErrorKind};
use crate::consts::{ACCEPT, HOST, SERVER, USER_AGENT};

pub mod names;
pub mod values;

pub use names::{header_consts, HeaderKind, HeaderName};
pub use values::HeaderValue;

pub struct Header;

impl Header {
    /// Parses a string slice into a `HeaderName` and a `HeaderValue`.
    pub fn parse(line: &str) -> NetResult<(HeaderName, HeaderValue)> {
        let mut tokens = line.splitn(2, ':').map(str::trim);

        let (Some(name), Some(value)) = (tokens.next(), tokens.next()) else {
            return Err(ParseErrorKind::Header)?;
        };

        let hdr_name = HeaderName::from(name);
        let hdr_value = HeaderValue::from(value);

        Ok((hdr_name, hdr_value))
    }
}

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
        self.insert(HOST, host.into());
    }

    /// Inserts the default User-Agent header.
    pub fn insert_user_agent(&mut self) {
        let agent = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
        self.insert(USER_AGENT, agent.into());
    }

    /// Inserts an Accept header with a value of "*/*".
    pub fn insert_accept_all(&mut self) {
        self.insert(ACCEPT, "*/*".into());
    }

    /// Inserts the default Server header.
    pub fn insert_server(&mut self) {
        let server = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
        self.insert(SERVER, server.into());
    }
}
