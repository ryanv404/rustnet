use std::fmt::{Display, Formatter, Result as FmtResult};
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct HeaderValue(Vec<u8>);

impl Display for HeaderValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", String::from_utf8_lossy(self.as_bytes()))
    }
}

impl From<&str> for HeaderValue {
    fn from(s: &str) -> Self {
        Self(Vec::from(s))
    }
}

impl From<&[u8]> for HeaderValue {
    fn from(bytes: &[u8]) -> Self {
        Self(Vec::from(bytes))
    }
}

impl HeaderValue {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[must_use]
    pub fn cache_control_from_path(path: &Path) -> Self {
        path.extension().map_or_else(
            || Self(Vec::from("no-cache")),
            |ext| {
                Self(Vec::from(match ext.to_str() {
                    // Allow caching of the favicon for 1 day.
                    Some("ico") => "max-age=86400",
                    // Don't cache HTML pages, etc during development.
                    Some(_) | None => "no-cache",
                }))
            },
        )
    }

    #[must_use]
    pub fn content_type_from_path(path: &Path) -> Self {
        path.extension().map_or_else(
            || Self(Vec::from("text/plain; charset=UTF-8")),
            |ext| {
                Self(Vec::from(match ext.to_str() {
                    Some("html" | "htm") => "text/html; charset=UTF-8",
                    Some("ico") => "image/x-icon",
                    Some("txt") => "text/plain; charset=UTF-8",
                    Some("json") => "application/json",
                    Some("jpg" | "jpeg") => "image/jpeg",
                    Some("png") => "image/png",
                    Some("pdf") => "application/pdf",
                    Some("gif") => "image/gif",
                    _ => "text/plain",
                }))
            },
        )
    }
}
