use std::borrow::{Borrow, Cow};
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use crate::{Method, NetParseError};
use crate::utils;

/// A respresentation of the message body.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Body {
    Empty,
    Xml(Cow<'static, str>),
    Html(Cow<'static, str>),
    Json(Cow<'static, str>),
    Text(Cow<'static, str>),
    Bytes(Cow<'static, [u8]>),
    Favicon(Cow<'static, [u8]>),
}

impl Default for Body {
    fn default() -> Self {
        Self::Empty
    }
}

impl Display for Body {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty | Self::Bytes(_) | Self::Favicon(_) => Ok(()),
            Self::Xml(ref s)
                | Self::Html(ref s)
                | Self::Json(ref s)
                | Self::Text(ref s) => write!(f, "{}", s.trim_end()),
        }
    }
}

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Body::Empty"),
            Self::Bytes(_) => write!(f, "Body::Bytes(...)"),
            Self::Favicon(_) => write!(f, "Body::Favicon(...)"),
            Self::Xml(ref s) => write!(f, "Body::Xml({:?})", s.trim_end()),
            Self::Html(ref s) => write!(f, "Body::Html({:?})", s.trim_end()),
            Self::Json(ref s) => write!(f, "Body::Json({:?})", s.trim_end()),
            Self::Text(ref s) => write!(f, "Body::Text({:?})", s.trim_end()),
        }
    }
}

impl From<&'static str> for Body {
    fn from(body: &'static str) -> Self {
        Self::Text(Cow::Borrowed(body))
    }
}

impl From<&'static [u8]> for Body {
    fn from(bytes: &'static [u8]) -> Self {
        Self::Bytes(Cow::Borrowed(bytes))
    }
}

impl From<String> for Body {
    fn from(body: String) -> Self {
        Self::Text(Cow::Owned(body))
    }
}

impl From<Vec<u8>> for Body {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(Cow::Owned(bytes))
    }
}

impl TryFrom<Target> for Body {
    type Error = NetParseError;

    fn try_from(target: Target) -> Result<Self, Self::Error> {
        match target {
            Target::Empty | Target::NotFound => Ok(Self::Empty),
            Target::Shutdown => Ok("Server is shutting down.".into()),
            Target::Xml(s) => Ok(Self::Xml(s)),
            Target::Html(s) => Ok(Self::Html(s)),
            Target::Json(s) => Ok(Self::Json(s)),
            Target::Text(s) => Ok(Self::Text(s)),
            Target::Bytes(b) => Ok(Self::Bytes(b)),
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
            Self::Xml(_)
                | Self::Html(_)
                | Self::Json(_)
                | Self::Text(_)
        )
    }

    /// Returns a reference to the data contained within the `Body`
    /// instance, if present.
    #[must_use]
    pub fn get_ref(&self) -> Option<&[u8]> {
        match self {
            Self::Empty => None,
            Self::Xml(s) | Self::Html(s) | Self::Json(s) | Self::Text(s) => {
                let body: &str = s.borrow();
                Some(body.as_bytes())
            },
            Self::Bytes(buf) | Self::Favicon(buf) => {
             Some(buf.borrow())
            },
        }
    }

    /// Returns the `Body` as a bytes slice.
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
            Self::Xml(_) => Some("application/xml"),
            Self::Favicon(_) => Some("image/x-icon"),
            Self::Json(_) => Some("application/json"),
            Self::Html(_) => Some("text/html; charset=utf-8"),
            Self::Text(_) => Some("text/plain; charset=utf-8"),
            Self::Bytes(_) => Some("application/octet-stream"),
        }
    }

    /// Returns a new `Body` instance from a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the file at `filepath` fails.
    pub fn from_filepath(filepath: &Path) -> Result<Self, NetParseError> {
        let data = fs::read(filepath).map_err(|_| NetParseError::Body)?;

        match utils::get_extension(filepath) {
            Some("ico") => Ok(Self::Favicon(data.into())),
            Some("xml") => {
                let body = String::from_utf8_lossy(&data);
                Ok(Self::Xml(body.into_owned().into()))
            },
            Some("txt") => {
                let body = String::from_utf8_lossy(&data);
                Ok(Self::Text(body.into_owned().into()))
            },
            Some("json") => {
                let body = String::from_utf8_lossy(&data);
                Ok(Self::Json(body.into_owned().into()))
            },
            Some("html" | "htm") => {
                let body = String::from_utf8_lossy(&data);
                Ok(Self::Html(body.into_owned().into()))
            },
            Some(_) | None => Ok(Self::Bytes(data.into())),
        }
    }

    /// Parses a `Body` from a bytes slice and a Content-Type header value.
    #[must_use]
    pub fn from_content_type(buf: &[u8], content_type: &str) -> Self {
        if buf.is_empty() || content_type.is_empty() {
            return Self::Empty;
        }

        match content_type.trim_start() {
            s if s.starts_with("text/html") => {
                let body = String::from_utf8_lossy(buf);
                Self::Html(body.into_owned().into())
            },
            s if s.starts_with("text/plain") => {
                let body = String::from_utf8_lossy(buf);
                Self::Text(body.into_owned().into())
            },
            s if s.starts_with("application/xml") => {
                let body = String::from_utf8_lossy(buf);
                Self::Xml(body.into_owned().into())
            },
            s if s.starts_with("application/json") => {
                let body = String::from_utf8_lossy(buf);
                Self::Json(body.into_owned().into())
            },
            s if s.starts_with("image/x-icon") => {
                Self::Favicon(buf.to_vec().into())
            },
            _ => Self::Bytes(buf.to_vec().into()),
        }
    }

    /// Returns true if a body is not permitted based on the given `Method`
    /// and status code.
    #[must_use]
    pub fn should_be_empty(status_code: u16, method: &Method) -> bool {
        match status_code {
            // 1xx (Informational), 204 (No Content), and 304 (Not Modified).
            100..=199 | 204 | 304 => true,
            // CONNECT responses with a 2xx (Success) status.
            200..=299 if matches!(method, Method::Connect) => true,
            // HEAD responses.
            _ if matches!(method, Method::Head) => true,
            _ => false,
        }
    }
}

/// Target resources served by routes in a `Router`.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Empty,
    Shutdown,
    NotFound,
    Xml(Cow<'static, str>),
    Html(Cow<'static, str>),
    Json(Cow<'static, str>),
    Text(Cow<'static, str>),
    Bytes(Cow<'static, [u8]>),
    File(Cow<'static, Path>),
    Favicon(Cow<'static, Path>),
}

impl Default for Target {
    fn default() -> Self {
        Self::Empty
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Target::Empty"),
            Self::Shutdown => write!(f, "Target::Shutdown"),
            Self::NotFound => write!(f, "Target::NotFound"),
            Self::Bytes(_) => write!(f, "Target::Bytes(...)"),
            Self::Xml(ref s) => write!(f, "Target::Xml({})", s.trim_end()),
            Self::Html(ref s) => write!(f, "Target::Html({})", s.trim_end()),
            Self::Json(ref s) => write!(f, "Target::Json({})", s.trim_end()),
            Self::Text(ref s) => write!(f, "Target::Text({})", s.trim_end()),
            Self::File(ref p) => write!(f, "Target::File({})", p.display()),
            Self::Favicon(ref p) => write!(f, "Target::Favicon({})", p.display()),
        }
    }
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Target::Empty"),
            Self::Shutdown => write!(f, "Target::Shutdown"),
            Self::NotFound => write!(f, "Target::NotFound"),
            Self::Bytes(_) => write!(f, "Target::Bytes(...)"),
            Self::Xml(ref s) => write!(f, "Target::Xml({:?})", s.trim_end()),
            Self::Html(ref s) => write!(f, "Target::Html({:?})", s.trim_end()),
            Self::Json(ref s) => write!(f, "Target::Json({:?})", s.trim_end()),
            Self::Text(ref s) => write!(f, "Target::Text({:?})", s.trim_end()),
            Self::File(ref p) => write!(f, "Target::File({:?})", p.display()),
            Self::Favicon(ref p) => write!(f, "Target::Favicon({:?})", p.display()),
        }
    }
}

impl From<&'static str> for Target {
    fn from(text: &'static str) -> Self {
        Self::Text(Cow::Borrowed(text))
    }
}

impl From<&'static [u8]> for Target {
    fn from(bytes: &'static [u8]) -> Self {
        Self::Bytes(Cow::Borrowed(bytes))
    }
}

impl From<String> for Target {
    fn from(text: String) -> Self {
        Self::Text(Cow::Owned(text))
    }
}

impl From<Vec<u8>> for Target {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(Cow::Owned(bytes))
    }
}

impl From<&'static Path> for Target {
    fn from(path: &'static Path) -> Self {
        Self::File(Cow::Borrowed(path))
    }
}

impl From<PathBuf> for Target {
    fn from(path: PathBuf) -> Self {
        Self::File(Cow::Owned(path))
    }
}

impl Target {
    /// Returns a new `Target::Empty` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the target type is `Target::Empty`.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns true if the target type is `Target::NotFound`.
    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }

    /// Returns true if the target type is `Target::Shutdown`.
    #[must_use]
    pub const fn is_shutdown(&self) -> bool {
        matches!(self, Self::Shutdown)
    }

    /// Returns true if the target type is `Target::Text`.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns true if the target type is `Target::Json`.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns true if the target type is `Target::Html`.
    #[must_use]
    pub const fn is_html(&self) -> bool {
        matches!(self, Self::Html(_))
    }

    /// Returns true if the target type is `Target::Xml`.
    #[must_use]
    pub const fn is_xml(&self) -> bool {
        matches!(self, Self::Xml(_))
    }

    /// Returns true if the target type is `Target::File`.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }

    /// Returns true if the target type is `Target::Bytes`.
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    /// Returns true if the target type is `Target::Favicon`.
    #[must_use]
    pub const fn is_favicon(&self) -> bool {
        matches!(self, Self::Favicon(_))
    }

    /// Returns the `Target` as a Content-Type header value, if possible.
    #[must_use]
    pub fn as_content_type(&self) -> Option<&str> {
        match self {
            Self::Empty | Self::NotFound => None,
            Self::Xml(_) => Some("application/xml"),
            Self::Html(_) => Some("text/html; charset=utf-8"),
            Self::Json(_) => Some("application/json"),
            Self::Text(_) | Self::Shutdown => Some("text/plain; charset=utf-8"),
            Self::Bytes(_) => Some("application/octet-stream"),
            Self::File(ref path) | Self::Favicon(ref path) => {
                utils::content_type_from_ext(path)
            },
        }
    }
}
