use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::path::Path;
use std::str;

use crate::{NetError, NetResult, Target};
use crate::util::get_extension;

/// A respresentation of the message body.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Body {
    Empty,
    Text(Vec<u8>),
    Html(Vec<u8>),
    Json(Vec<u8>),
    Xml(Vec<u8>),
    Image(Vec<u8>),
    Bytes(Vec<u8>),
    Favicon(Vec<u8>),
}

impl Default for Body {
    fn default() -> Self {
        Self::Empty
    }
}

impl Display for Body {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_printable() {
            if let Some(body) = self.get_ref() {
                write!(f, "{}", String::from_utf8_lossy(body))?;
            }
        }

        Ok(())
    }
}

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Body::Empty"),
            Self::Xml(_) => write!(f, "Body::Xml({})", self.to_string()),
            Self::Text(_) => write!(f, "Body::Text({})", self.to_string()),
            Self::Html(_) => write!(f, "Body::Html({})", self.to_string()),
            Self::Json(_) => write!(f, "Body::Json({})", self.to_string()),
            Self::Image(_) => write!(f, "Body::Image(...)"),
            Self::Bytes(_) => write!(f, "Body::Bytes(...)"),
            Self::Favicon(_) => write!(f, "Body::Favicon(...)"),
        }
    }
}

impl TryFrom<Target> for Body {
    type Error = NetError;

    fn try_from(target: Target) -> NetResult<Self> {
        match target {
            Target::Empty | Target::NotFound => Ok(Self::Empty),
            Target::Xml(bytes) => Ok(Self::Xml(bytes.to_vec())),
            Target::Text(bytes) => Ok(Self::Text(bytes.to_vec())),
            Target::Html(bytes) => Ok(Self::Html(bytes.to_vec())),
            Target::Json(bytes) => Ok(Self::Json(bytes.to_vec())),
            Target::Bytes(bytes) => Ok(Self::Bytes(bytes.to_vec())),
            Target::File(path) => Ok(Self::from_filepath(path)?),
            Target::Favicon(path) => Ok(Self::from_filepath(path)?),
        }
    }
}

impl Body {
    /// Returns a new `Body::Empty` instance.
    #[must_use]
    pub const fn new() -> Self {
        Self::Empty
    }

    /// Returns true if the body type is `Body::Empty`.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns true if the body type is `Body::Text`.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns true if the body type is `Body::Json`.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns true if the body type is `Body::Html`.
    #[must_use]
    pub const fn is_html(&self) -> bool {
        matches!(self, Self::Html(_))
    }

    /// Returns true if the body type is `Body::Xml`.
    #[must_use]
    pub const fn is_xml(&self) -> bool {
        matches!(self, Self::Xml(_))
    }

    /// Returns true if the body type is `Body::Image`.
    #[must_use]
    pub const fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    /// Returns true if the body type is `Body::Favicon`.
    #[must_use]
    pub const fn is_favicon(&self) -> bool {
        matches!(self, Self::Favicon(_))
    }

    /// Returns true if the body type is `Body::Favicon`.
    #[must_use]
    pub const fn is_printable(&self) -> bool {
        matches!(self, Self::Text(_) | Self::Html(_) | Self::Json(_)
            | Self::Xml(_))
    }

    /// Returns a reference to the data contained within the `Body`
    /// instance, if present.
    #[must_use]
    pub fn get_ref(&self) -> Option<&[u8]> {
        match self {
            Self::Empty => None,
            Self::Image(buf) | Self::Bytes(buf) | Self::Favicon(buf)
                | Self::Text(buf) | Self::Html(buf) | Self::Json(buf)
                | Self::Xml(buf) => Some(buf.as_slice()),
        }
    }

    /// Returns the `Body` as a slice of bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.get_ref().unwrap_or(&b""[..])
    }

    /// Returns the length of the `Body`.
    #[must_use]
    pub fn len(&self) -> usize {
        self.get_ref().map_or(0, |buf| buf.len())
    }

    /// Returns the `Body` as a Content-Type header value, if possible.
    #[must_use]
    pub fn as_content_type(&self) -> Option<&'static str> {
        match self {
            Self::Empty => None,
            Self::Text(_) => Some("text/plain; charset=utf-8"),
            Self::Html(_) => Some("text/html; charset=utf-8"),
            Self::Json(_) => Some("application/json"),
            Self::Xml(_) => Some("application/xml"),
            Self::Bytes(_) => Some("application/octet-stream"),
            Self::Favicon(_) => Some("image/x-icon"),
            Self::Image(_) => Some("image"),
        }
    }

    /// Returns a new `Body` instance from a file path.
    pub fn from_filepath(filepath: &Path) -> NetResult<Self> {
        let data = fs::read(filepath)?;

        match get_extension(filepath) {
            None => Ok(Self::Bytes(data)),
            Some("txt") => Ok(Self::Text(data)),
            Some("html" | "htm") => Ok(Self::Html(data)),
            Some("json") => Ok(Self::Json(data)),
            Some("xml") => Ok(Self::Xml(data)),
            Some("ico") => Ok(Self::Favicon(data)),
            Some("jpg" | "jpeg" | "png" | "gif") => Ok(Self::Image(data)),
            Some(_) => Ok(Self::Bytes(data)),
        }
    }
}
