use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::SocketAddr;
use std::str::{self, FromStr};

use crate::{Body, NetError, NetResult, DEFAULT_NAME};
use crate::style::colors::{BLUE, CYAN, RESET};
use crate::utils;

pub mod names;
pub mod values;

pub use names::HeaderName;
pub use values::HeaderValue;

/// A convenience type containing a single header's name and value.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Header(pub HeaderName, pub HeaderValue);

impl FromStr for Header {
    type Err = NetError;

    fn from_str(input: &str) -> NetResult<Self> {
        Self::try_from(input.as_bytes())
    }
}

impl TryFrom<&[u8]> for Header {
    type Error = NetError;

    fn try_from(input: &[u8]) -> NetResult<Self> {
        // Split header into name and value slices at the ':'.
        let (name, value) = utils::trim(input)
            .iter()
            .position(|&b| b == b':')
            .ok_or(NetError::BadHeader)
            .map(|colon| input.split_at(colon))?;

        // Remove the colon used above by `split_at` and trim whitespace.
        let value = match value.strip_prefix(b":") {
            Some(strip) => utils::trim_start(strip),
            None if value.is_empty() => &[][..],
            None => utils::trim_start(value),
        };

        let name = HeaderName::try_from(name)?;
        let value = HeaderValue::from(value);

        Ok(Self(name, value))
    }
}

impl Header {
    /// Returns a reference to the `HeaderName`.
    #[must_use]
    pub const fn name(&self) -> &HeaderName {
        &self.0
    }

    /// Returns a reference to the `HeaderValue`.
    #[must_use]
    pub const fn value(&self) -> &HeaderValue {
        &self.1
    }

    /// Returns the inner `HeaderName` and `HeaderValue` as a tuple.
    #[must_use]
    pub fn into_tuple(self) -> (HeaderName, HeaderValue) {
        (self.0.clone(), self.1)
    }
}

/// A mapping of `HeaderNames` to `HeaderValues`.
#[derive(Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Headers(pub BTreeMap<HeaderName, HeaderValue>);

impl Display for Headers {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for (name, value) in &self.0 {
            writeln!(f, "{name}: {value}")?;
        }

        Ok(())
    }
}

impl FromStr for Headers {
    type Err = NetError;

    fn from_str(headers: &str) -> NetResult<Self> {
        Self::try_from(headers.as_bytes())
    }
}

impl TryFrom<&[u8]> for Headers {
    type Error = NetError;

    fn try_from(input: &[u8]) -> NetResult<Self> {
        input.split_inclusive(|&b| b == b'\n')
            .map(utils::trim)
            .take_while(|line| !line.is_empty())
            .map(Header::try_from)
            .collect::<NetResult<Self>>()
    }
}

impl FromIterator<Header> for Headers {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Header>,
    {
        let map = iter
            .into_iter()
            .map(Header::into_tuple)
            .collect::<BTreeMap<HeaderName, HeaderValue>>();

        Self(map)
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

        if !self.contains(&CONTENT_LENGTH) && !body.is_empty() {
            self.insert(CONTENT_LENGTH, body.len().into());
        }

        if !self.contains(&CONTENT_TYPE) && !body.is_empty() {
            if let Some(content_type) = body.as_content_type() {
                self.insert(CONTENT_TYPE, content_type.into());
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

        if !self.contains(&CONTENT_LENGTH) && !body.is_empty() {
            self.insert(CONTENT_LENGTH, body.len().into());
        }

        if !self.contains(&CONTENT_TYPE) && !body.is_empty() {
            if let Some(content_type) = body.as_content_type() {
                self.insert(CONTENT_TYPE, content_type.into());
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
