use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Result as FmtResult};

use crate::{NetResult, ParseErrorKind};

pub mod names;
pub mod values;

pub use names::{header_consts, HeaderKind, HeaderName};
pub use values::HeaderValue;

/// An object that represents a header field.
#[derive(Debug)]
pub struct Header {
    pub name: HeaderName,
    pub value: HeaderValue,
}

impl PartialEq<Self> for Header {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialEq<HeaderName> for Header {
    fn eq(&self, other: &HeaderName) -> bool {
        self.name == *other
    }
}

impl Eq for Header {}

impl PartialOrd<Header> for Header {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.name.partial_cmp(&other.name))
    }
}

impl PartialOrd<HeaderName> for Header {
    fn partial_cmp(&self, other: &HeaderName) -> Option<Ordering> {
        Some(self.name.partial_cmp(&other))
    }
}

impl Ord for Header {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}: {}", &self.name, &self.value)
    }
}

impl Header {
    /// Returns a new `Header` instance.
    pub fn new(name: HeaderName, value: HeaderValue) -> Self {
        Self { name, value }
    }

    /// Parses a string slice into a `Header` object.
    pub fn parse(line: &str) -> NetResult<Header> {
        let mut tokens = line
            .splitn(2, ':')
            .map(str::trim);

        let (Some(name), Some(value)) = (tokens.next(), tokens.next()) else {
            return Err(ParseErrorKind::Header)?;
        };

        let hdr_name = HeaderName::from(name);
        let hdr_value = HeaderValue::from(value);

        Ok(Self::new(hdr_name, hdr_value))
    }
}

