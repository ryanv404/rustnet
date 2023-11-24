use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::Path;

#[derive(Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct HeaderValue(pub Vec<u8>);

impl Display for HeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", String::from_utf8_lossy(self.as_bytes()))
    }
}

impl Debug for HeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		Debug::fmt(&self.to_string(), f)
	}
}

impl From<&str> for HeaderValue {
    fn from(s: &str) -> Self {
        Self(Vec::from(s))
    }
}

impl From<String> for HeaderValue {
    fn from(s: String) -> Self {
        Self(s.into_bytes())
    }
}

impl From<&[u8]> for HeaderValue {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl From<usize> for HeaderValue {
    fn from(num: usize) -> Self {
        let num = num.to_string();
        Self(Vec::from(num.as_str()))
    }
}

impl From<Vec<u8>> for HeaderValue {
    fn from(vec: Vec<u8>) -> Self {
        Self(vec)
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

	/// Infers the Content-Type value from a resource's file extension.
	/// Defaults to "text/plain" if the extension is not recognized.
    #[must_use]
    pub fn infer_content_type(path: &Path) -> Self {
        path.extension().map_or_else(
            || Self(Vec::from("text/plain")),
            |ext| {
                Self(Vec::from(match ext.to_str() {
                    Some("html" | "htm") => "text/html; charset=UTF-8",
                    Some("txt") => "text/plain; charset=UTF-8",
                    Some("json") => "application/json",
                    Some("pdf") => "application/pdf",
                    Some("ico") => "image/x-icon",
                    Some("jpg" | "jpeg") => "image/jpeg",
                    Some("png") => "image/png",
                    Some("gif") => "image/gif",
                    _ => "text/plain",
                }))
            },
        )
    }
}
