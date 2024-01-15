use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use crate::utils;

/// An HTTP header value.
#[derive(Clone, Default, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct HeaderValue(pub Vec<u8>);

impl Display for HeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for HeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for HeaderValue {
    fn from(value: &str) -> Self {
        Self(Vec::from(value.trim()))
    }
}

impl From<&[u8]> for HeaderValue {
    fn from(value: &[u8]) -> Self {
        Self(utils::trim(value).to_vec())
    }
}

impl From<usize> for HeaderValue {
    fn from(value: usize) -> Self {
        Self(value.to_string().into_bytes())
    }
}

impl HeaderValue {
    /// Returns the `HeaderValue` as a copy-on-write string slice.
    #[must_use]
    pub fn as_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }

    /// Returns the `HeaderValue` as a bytes slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}
