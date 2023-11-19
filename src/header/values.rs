use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::Path;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd)]
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

impl From<&[u8]> for HeaderValue {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl From<usize> for HeaderValue {
    fn from(num: usize) -> Self {
        Self(Vec::from(&*num.to_string()))
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
