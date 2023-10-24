use std::{borrow::Cow, fmt};

use crate::util::trim_whitespace;

#[macro_use]
pub mod macros;
pub mod names;

pub use names::HeaderName;

#[derive(Clone, Eq, PartialEq)]
pub struct Header {
    pub name: HeaderName,
    pub value: Vec<u8>,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name(), self.value_str())
    }
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Header")
            .field("name", self.name())
            .field("value", &self.value_str())
            .finish()
    }
}

impl Header {
    pub fn new(name: &[u8], value: &[u8]) -> Self {
        let name = HeaderName::from(trim_whitespace(name));
        let value = trim_whitespace(value).to_owned();
        Self { name, value }
    }

    pub fn name(&self) -> &HeaderName {
        &self.name
    }

    pub fn value_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(&self.value)
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        format!("{}: {}\r\n", self.name(), self.value_str()).into_bytes()
    }
}
