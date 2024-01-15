use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::SocketAddr;
use std::str::{self, FromStr};

use crate::{Body, NetParseError, DEFAULT_NAME};
use crate::style::colors::{BLUE, CYAN, RESET};
use crate::utils;

pub mod names;
pub mod values;

pub use names::HeaderName;
pub use values::HeaderValue;

/// A mapping of `HeaderNames` to `HeaderValues`.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Headers(pub BTreeMap<HeaderName, HeaderValue>);

impl Display for Headers {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for (name, value) in self.0.iter() {
            writeln!(f, "{name}: {value}")?;
        }

        Ok(())
    }
}

impl FromStr for Headers {
    type Err = NetParseError;

    fn from_str(headers: &str) -> Result<Self, Self::Err> {
        Self::try_from(headers.as_bytes())
    }
}

impl TryFrom<&[u8]> for Headers {
    type Error = NetParseError;

    fn try_from(headers: &[u8]) -> Result<Self, Self::Error> {
        let mut headers_map = Self::new();

        let mut lines = utils::trim_start(headers).split(|b| *b == b'\n');

        while let Some(line) = lines.next() {
            let line = utils::trim(line);

            if line.is_empty() {
                break;
            }

            let mut parts = line.splitn(2, |b| *b == b':');

            let name = parts
                .next()
                .ok_or(NetParseError::Header)
                .and_then(HeaderName::try_from)?;

            let value = parts
                .next()
                .ok_or(NetParseError::Header)
                .map(HeaderValue::from)?;

            headers_map.insert(name, value);
        }

        Ok(headers_map)
    }
}

impl Headers {
    /// Returns a new `Headers` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the `HeaderValue` that is mapped to the given `HeaderName`,
    /// if present.
    #[must_use]
    pub fn get(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.0.get(name)
    }

    /// Returns the number of header field entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if there are no header entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes a header field entry from the `Headers` map.
    pub fn remove(&mut self, name: &HeaderName) {
        self.0.remove(name);
    }

    /// Returns true if the header name represented by `HeaderName` is present.
    #[must_use]
    pub fn contains(&self, name: &HeaderName) -> bool {
        self.0.contains_key(name)
    }

    /// Appends the entries from another `Headers` collection to this one.
    pub fn append(&mut self, other: &mut Self) {
        self.0.append(&mut other.0);
    }

    /// Inserts a new header entry from the given name and value or updates
    /// the value if an entry with the same name was already present.
    pub fn header(&mut self, name: &str, value: &[u8]) {
        self.insert(HeaderName::from(name), HeaderValue::from(value));
    }

    /// Inserts a new header entry from the given `HeaderName` and
    /// `HeaderValue` or updates the value if the key was already present.
    pub fn insert(&mut self, name: HeaderName, value: HeaderValue) {
        self.0.insert(name, value);
    }

    /// Inserts a sensible set default of request headers.
    pub fn default_request_headers(
        &mut self,
        body: &Body,
        remote_addr: Option<SocketAddr>
    ) {
        use crate::headers::names::{
            ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, DATE, HOST, USER_AGENT,
        };

        if !self.contains(&ACCEPT) {
            self.insert(ACCEPT, "*/*".into());
        }

        if !self.contains(&CONTENT_LENGTH) {
            if !body.is_empty() {
                self.insert(CONTENT_LENGTH, body.len().into());
            }
        }

        if !self.contains(&CONTENT_TYPE) {
            if !body.is_empty() {
                if let Some(content_type) = body.as_content_type() {
                    self.insert(CONTENT_TYPE, content_type.into());
                }
            }
        }

        if !self.contains(&DATE) {
            if let Some(date_value) = utils::get_datetime() {
                self.insert(DATE, date_value);
            }
        }

        if !self.contains(&HOST) {
            if let Some(addr) = remote_addr {
                let addr = addr.to_string();
                self.insert(HOST, addr.as_str().into());
            }
        }

        if !self.contains(&USER_AGENT) {
            self.insert(USER_AGENT, DEFAULT_NAME.into());
        }
    }

    /// Inserts a sensible set of default response headers.
    pub fn default_response_headers(&mut self, body: &Body) {
        use crate::headers::names::{
            CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE, DATE, SERVER,
        };

        if !self.contains(&CACHE_CONTROL) {
            if body.is_favicon() {
                // Allow caching of favicons for 1 week.
                self.insert(CACHE_CONTROL, "max-age=604800".into());
            } else {
                self.insert(CACHE_CONTROL, "no-cache".into());
            }
        }

        if !self.contains(&CONTENT_LENGTH) {
            if !body.is_empty() {
                self.insert(CONTENT_LENGTH, body.len().into());
            }
        }

        if !self.contains(&CONTENT_TYPE) {
            if !body.is_empty() {
                if let Some(content_type) = body.as_content_type() {
                    self.insert(CONTENT_TYPE, content_type.into());
                }
            }
        }

        if !self.contains(&DATE) {
            if let Some(date_value) = utils::get_datetime() {
                self.insert(DATE, date_value);
            }
        }

        if !self.contains(&SERVER) {
            self.insert(SERVER, DEFAULT_NAME.into());
        }
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
