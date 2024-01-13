use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::net::SocketAddr;

use crate::utils::Trim;

#[derive(Clone, Default, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct HeaderValue(pub Vec<u8>);

impl Display for HeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", String::from_utf8_lossy(self.as_bytes()))
    }
}

impl Debug for HeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:?}", self.to_string())
    }
}

impl From<&str> for HeaderValue {
    fn from(value: &str) -> Self {
        Self(Vec::from(value.trim()))
    }
}

impl From<String> for HeaderValue {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl From<&[u8]> for HeaderValue {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.trim().to_vec())
    }
}

impl From<usize> for HeaderValue {
    fn from(num: usize) -> Self {
        let num = num.to_string();
        Self(num.into_bytes())
    }
}

impl From<Vec<u8>> for HeaderValue {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes.as_slice().trim().to_vec())
    }
}

impl From<SocketAddr> for HeaderValue {
    fn from(sock: SocketAddr) -> Self {
        let sock = sock.to_string();
        Self(sock.into_bytes())
    }
}

impl HeaderValue {
    /// Constructs a `HeaderValue` from a bytes slice.
    #[must_use]
    pub fn new(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }

    /// Returns the header field value as a bytes slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns the header field value as a copy-on-write string slice.
    #[must_use]
    pub fn as_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.0)
    }
}
