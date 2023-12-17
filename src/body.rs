use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::path::Path;

use crate::{NetError, NetResult};

/// A respresentation of the body content type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Body {
    Empty,
    Text(String),
    Html(String),
    Json(String),
    Xml(String),
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
    #[allow(clippy::match_same_arms)]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => Ok(()),
            Self::Text(s) => write!(f, "{s}"),
            Self::Html(s) => write!(f, "{s}"),
            Self::Json(s) => write!(f, "{s}"),
            Self::Xml(s) => write!(f, "{s}"),
            Self::Image(_) => write!(f, "Image {{ ... }}"),
            Self::Bytes(_) => write!(f, "Bytes {{ ... }}"),
            Self::Favicon(_) => write!(f, "Favicon {{ ... }}"),
        }
    }
}

impl TryFrom<&Path> for Body {
    type Error = NetError;

    fn try_from(filepath: &Path) -> NetResult<Self> {
        match filepath.extension() {
            None => {
                let body = fs::read(filepath)?;
                Ok(Self::Bytes(body))
            }
            Some(ext) => match ext.to_str() {
                Some("txt") => {
                    let content = fs::read_to_string(filepath)?;
                    Ok(Self::Text(content))
                }
                Some("html" | "htm") => {
                    let content = fs::read_to_string(filepath)?;
                    Ok(Self::Html(content))
                }
                Some("json") => {
                    let content = fs::read_to_string(filepath)?;
                    Ok(Self::Json(content))
                }
                Some("xml") => {
                    let content = fs::read_to_string(filepath)?;
                    Ok(Self::Xml(content))
                }
                Some("ico") => {
                    let buf = fs::read(filepath)?;
                    Ok(Self::Favicon(buf))
                }
                Some("jpg" | "jpeg" | "png" | "gif") => {
                    let buf = fs::read(filepath)?;
                    Ok(Self::Image(buf))
                }
                _ => {
                    let buf = fs::read(filepath)?;
                    Ok(Self::Bytes(buf))
                }
            },
        }
    }
}

impl Body {
    /// Returns a default `Body` instance.
    #[must_use]
    pub const fn new() -> Self {
        Self::Empty
    }

    /// Returns true if the body type is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns true if the body type is JSON.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns true if the body type is HTML.
    #[must_use]
    pub const fn is_html(&self) -> bool {
        matches!(self, Self::Html(_))
    }

    /// Returns true if the body type is XML.
    #[must_use]
    pub const fn is_xml(&self) -> bool {
        matches!(self, Self::Xml(_))
    }

    /// Returns true if the URI target is a vector of bytes.
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Self::Image(_) | Self::Bytes(_) | Self::Favicon(_))
    }

    /// Returns true if the body contains a alphanumeric data.
    #[must_use]
    pub const fn is_alphanumeric(&self) -> bool {
        !self.is_bytes()
    }

    /// Returns the body data as a bytes slice.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Empty => &b""[..],
            Self::Text(ref s) => s.as_bytes(),
            Self::Html(ref s) => s.as_bytes(),
            Self::Json(ref s) => s.as_bytes(),
            Self::Xml(ref s) => s.as_bytes(),
            Self::Image(ref buf) => buf.as_slice(),
            Self::Bytes(ref buf) => buf.as_slice(),
            Self::Favicon(ref buf) => buf.as_slice(),
        }
    }

    /// Returns the size of the body data as number of bytes.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Text(ref s) => s.len(),
            Self::Html(ref s) => s.len(),
            Self::Json(ref s) => s.len(),
            Self::Xml(ref s) => s.len(),
            Self::Image(ref buf) => buf.len(),
            Self::Bytes(ref buf) => buf.len(),
            Self::Favicon(ref buf) => buf.len(),
        }
    }
}
