use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::path::Path;
use std::str;

use crate::util::get_extension;
use crate::{NetError, NetResult, Target};

/// A respresentation of the message body.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Body {
    Empty,
    Text(Vec<u8>),
    Html(Vec<u8>),
    Json(Vec<u8>),
    Xml(Vec<u8>),
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
        match self {
            Self::Empty | Self::Bytes(_) | Self::Favicon(_) => {},
            Self::Text(ref buf)
                | Self::Html(ref buf)
                | Self::Xml(ref buf)
                | Self::Json(ref buf) =>
            {
                let body = String::from_utf8_lossy(buf);
                write!(f, "{}", body.trim_end())?;
            },
        }

        Ok(())
    }
}

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Body::Empty"),
            Self::Bytes(_) => write!(f, "Body::Bytes(...)"),
            Self::Favicon(_) => write!(f, "Body::Favicon(...)"),
            Self::Xml(buf) => {
                write!(f, "Body::Xml({})", String::from_utf8_lossy(buf))
            },
            Self::Text(buf) => {
                write!(f, "Body::Text({})", String::from_utf8_lossy(buf))
            },
            Self::Html(buf) => {
                write!(f, "Body::Html({})", String::from_utf8_lossy(buf))
            },
            Self::Json(buf) => {
                write!(f, "Body::Json({})", String::from_utf8_lossy(buf))
            },
        }
    }
}

impl TryFrom<Target> for Body {
    type Error = NetError;

    fn try_from(target: Target) -> NetResult<Self> {
        match target {
            Target::Empty | Target::NotFound => Ok(Self::Empty),
            Target::Shutdown => {
                Ok(Self::Text("Server is shutting down.".into()))
            },
            Target::Xml(s) => Ok(Self::Xml(s.into_bytes())),
            Target::Text(s) => Ok(Self::Text(s.into_bytes())),
            Target::Html(s) => Ok(Self::Html(s.into_bytes())),
            Target::Json(s) => Ok(Self::Json(s.into_bytes())),
            Target::Bytes(ref bytes) => Ok(Self::Bytes(bytes.clone())),
            Target::File(ref path) => Ok(Self::from_filepath(path)?),
            Target::Favicon(ref path) => Ok(Self::from_filepath(path)?),
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

    /// Returns true if the body type is `Body::Favicon`.
    #[must_use]
    pub const fn is_favicon(&self) -> bool {
        matches!(self, Self::Favicon(_))
    }

    /// Returns true if the body type is `Body::Favicon`.
    #[must_use]
    pub const fn is_printable(&self) -> bool {
        matches!(
            self,
            Self::Text(_) | Self::Html(_) | Self::Json(_) | Self::Xml(_)
        )
    }

    /// Returns a reference to the data contained within the `Body`
    /// instance, if present.
    #[must_use]
    pub fn get_ref(&self) -> Option<&[u8]> {
        match self {
            Self::Empty => None,
            Self::Bytes(buf)
                | Self::Favicon(buf)
                | Self::Text(buf)
                | Self::Html(buf)
                | Self::Json(buf)
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
        self.get_ref().map_or(0, <[u8]>::len)
    }

    /// Returns the `Body` as a Content-Type header value, if possible.
    #[must_use]
    pub const fn as_content_type(&self) -> Option<&'static str> {
        match self {
            Self::Empty => None,
            Self::Text(_) => Some("text/plain; charset=utf-8"),
            Self::Html(_) => Some("text/html; charset=utf-8"),
            Self::Json(_) => Some("application/json"),
            Self::Xml(_) => Some("application/xml"),
            Self::Bytes(_) => Some("application/octet-stream"),
            Self::Favicon(_) => Some("image/x-icon"),
        }
    }

    /// Returns a new `Body` instance from a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the file at `filepath` fails.
    pub fn from_filepath(filepath: &Path) -> NetResult<Self> {
        let data = fs::read(filepath)?;

        match get_extension(filepath) {
            Some("txt") => Ok(Self::Text(data)),
            Some("html" | "htm") => Ok(Self::Html(data)),
            Some("json") => Ok(Self::Json(data)),
            Some("xml") => Ok(Self::Xml(data)),
            Some("ico") => Ok(Self::Favicon(data)),
            Some(_) | None => Ok(Self::Bytes(data)),
        }
    }

    #[must_use]
    pub fn from_content_type(buf: &[u8], content_type: &str) -> Self {
        let buf = buf.to_owned();

        match content_type.trim_start() {
            s if s.starts_with("text/html") => Self::Html(buf),
            s if s.starts_with("text/plain") => Self::Text(buf),
            s if s.starts_with("application/xml") => Self::Xml(buf),
            s if s.starts_with("application/json") => Self::Json(buf),
            s if s.starts_with("image/x-icon") => Self::Favicon(buf),
            _ => Self::Bytes(buf),
        }
    }
}
