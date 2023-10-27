use std::{
    fmt,
    path::Path,
    str::FromStr,
};

use crate::{NetError, NetResult};

pub mod names;

#[allow(clippy::module_name_repetitions)]
pub use names::HeaderName;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Header {
    pub name: HeaderName,
    pub value: String,
}

impl fmt::Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name.as_str(), self.value())
    }
}

impl FromStr for Header {
    type Err = NetError;

    fn from_str(input: &str) -> NetResult<Self> {
        let tokens: Vec<&str> = input.splitn(2, ':').collect();

        if tokens.len() == 2 {
            Ok(Self::new(tokens[0], tokens[1])?)
        } else {
            Err(NetError::ParseError("request header"))
        }
    }
}

impl Header {
    pub fn new(name: &str, value: &str) -> NetResult<Self> {
        let name = HeaderName::from_str(name)?;
        let value = value.trim().to_lowercase();
        Ok(Self { name, value })
    }

    #[must_use]
    pub const fn name(&self) -> &HeaderName {
        &self.name
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    #[must_use]
    pub fn default_headers() -> Vec<Self> {
        let cache_con = Self {
            name: HeaderName::CacheControl,
            value: "no-cache".to_string()
        };
        let cont_type = Self {
            name: HeaderName::ContentType,
            value: "text/plain; charset=UTF-8".to_string()
        };
        let cont_len = Self {
            name: HeaderName::ContentLength,
            value: "0".to_string()
        };

        vec![cache_con, cont_type, cont_len]
    }

    #[must_use]
    pub fn cache_control_from_path(path: &Path) -> Self {
        path.extension()
            .map_or_else(
                || Self {
                    name: HeaderName::CacheControl,
                    value: "no-cache".to_string()
                },
                |ext| Self {
                    name: HeaderName::CacheControl,
                    value: match ext.to_str() {
                        // Allow caching of the favicon for 1 day.
                        Some("ico") => "max-age=86400",
                        // Don't cache HTML pages, etc during development.
                        Some(_) | None => "no-cache",
                     }.to_string()
                })
    }

    #[must_use]
    pub fn content_type_from_path(path: &Path) -> Self {
        path.extension()
            .map_or_else(
                || Self {
                    name: HeaderName::ContentType,
                    value: "text/plain; charset=UTF-8".to_string()
                },
                |ext| Self {
                    name: HeaderName::ContentType,
                    value: match ext.to_str() {
                        Some("html" | "htm") => "text/html; charset=UTF-8",
                        Some("ico") => "image/x-icon",
                        Some("txt") => "text/plain; charset=UTF-8",
                        Some("json") => "application/json",
                        Some("jpg" | "jpeg") => "image/jpeg",
                        Some("png") => "image/png",
                        Some("pdf") => "application/pdf",
                        Some("gif") => "image/gif",
                        _ => "text/plain",
                    }.to_string()
                })
    }
}
