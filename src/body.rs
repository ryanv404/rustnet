use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use crate::{Method, NetParseError};
use crate::util;

/// A respresentation of the message body.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Body {
    Empty,
    Xml(Cow<'static, [u8]>),
    Html(Cow<'static, [u8]>),
    Json(Cow<'static, [u8]>),
    Text(Cow<'static, [u8]>),
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
            Self::Xml(ref buf)
                | Self::Html(ref buf)
                | Self::Json(ref buf)
                | Self::Text(ref buf) =>
            {
                let body = String::from_utf8_lossy(buf);
                write!(f, "{}", body.trim_end())
            },
        }
    }
}

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Body::Empty"),
            Self::Bytes(_) => write!(f, "Body::Bytes(...)"),
            Self::Favicon(_) => write!(f, "Body::Favicon(...)"),
            Self::Xml(ref buf) => {
                write!(f, "Body::Xml({:?})", String::from_utf8_lossy(buf))
            },
            Self::Html(ref buf) => {
                write!(f, "Body::Html({:?})", String::from_utf8_lossy(buf))
            },
            Self::Json(ref buf) => {
                write!(f, "Body::Json({:?})", String::from_utf8_lossy(buf))
            },
            Self::Text(ref buf) => {
                write!(f, "Body::Text({:?})", String::from_utf8_lossy(buf))
            },
        }
    }
}

impl TryFrom<Target> for Body {
    type Error = NetParseError;

    fn try_from(target: Target) -> Result<Self, Self::Error> {
        match target {
            Target::Empty | Target::NotFound => Ok(Self::Empty),
            Target::Shutdown => {
                Ok(Self::Text(Cow::Borrowed(b"Server is shutting down.")))
            },
            Target::Xml(b) => Ok(Self::Xml(b.into())),
            Target::Html(b) => Ok(Self::Html(b.into())),
            Target::Json(b) => Ok(Self::Json(b.into())),
            Target::Text(b) => Ok(Self::Text(b.into())),
            Target::Bytes(b) => Ok(Self::Bytes(b.into())),
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
                | Self::Xml(buf)
                | Self::Html(buf)
                | Self::Json(buf)
                | Self::Text(buf)
                | Self::Bytes(buf)
                | Self::Favicon(buf) => Some(buf),
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

        match util::get_extension(filepath) {
            Some("xml") => Ok(Self::Xml(data.into())),
            Some("txt") => Ok(Self::Text(data.into())),
            Some("json") => Ok(Self::Json(data.into())),
            Some("ico") => Ok(Self::Favicon(data.into())),
            Some("html" | "htm") => Ok(Self::Html(data.into())),
            Some(_) | None => Ok(Self::Bytes(data.into())),
        }
    }

    #[must_use]
    pub fn from_content_type(buf: &[u8], content_type: &str) -> Self {
        let buf = buf.to_owned();

        match content_type.trim_start() {
            s if s.starts_with("text/html") => Self::Html(buf.into()),
            s if s.starts_with("text/plain") => Self::Text(buf.into()),
            s if s.starts_with("application/xml") => Self::Xml(buf.into()),
            s if s.starts_with("image/x-icon") => Self::Favicon(buf.into()),
            s if s.starts_with("application/json") => Self::Json(buf.into()),
            _ => Self::Bytes(buf.into()),
        }
    }

    /// Returns true if a body is not permitted based on the given `Method`
    /// and status code.
    #[must_use]
    pub fn should_be_empty(code: u16, method: Method) -> bool {
        match code {
            // 1xx (Informational), 204 (No Content), and 304 (Not Modified).
            100..=199 | 204 | 304 => true,
            // CONNECT responses with a 2xx (Success) status.
            200..=299 if method == Method::Connect => true,
            // HEAD responses.
            _ if method == Method::Head => true,
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
    Xml(Cow<'static, [u8]>),
    Html(Cow<'static, [u8]>),
    Json(Cow<'static, [u8]>),
    Text(Cow<'static, [u8]>),
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
            Self::Empty => write!(f, "Empty"),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::NotFound => write!(f, "Not Found"),
            Self::Bytes(_) => write!(f, "Bytes(...)"),
            Self::Xml(ref b) => {
                let target = String::from_utf8_lossy(b);
                write!(f, "Xml({})", target.trim_end())
            },
            Self::Html(ref b) => {
                let target = String::from_utf8_lossy(b);
                write!(f, "Html({})", target.trim_end())
            },
            Self::Json(ref b) => {
                let target = String::from_utf8_lossy(b);
                write!(f, "Json({})", target.trim_end())
            },
            Self::Text(ref b) => {
                let target = String::from_utf8_lossy(b);
                write!(f, "Text({})", target.trim_end())
            },
            Self::File(ref path) => {
                write!(f, "File({})", path.display())
            },
            Self::Favicon(ref path) => {
                write!(f, "Favicon({})", path.display())
            },
        }
    }
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::NotFound => write!(f, "NotFound"),
            Self::Bytes(_) => write!(f, "Bytes(...)"),
            Self::Xml(ref b) => {
                let target = String::from_utf8_lossy(b);
                write!(f, "Xml({:?})", target.trim_end())
            },
            Self::Html(ref b) => {
                let target = String::from_utf8_lossy(b);
                write!(f, "Html({:?})", target.trim_end())
            },
            Self::Json(ref b) => {
                let target = String::from_utf8_lossy(b);
                write!(f, "Json({:?})", target.trim_end())
            },
            Self::Text(ref b) => {
                let target = String::from_utf8_lossy(b);
                write!(f, "Text({:?})", target.trim_end())
            },
            Self::File(ref path) => {
                write!(f, "File({:?})", path.display())
            },
            Self::Favicon(ref path) => {
                write!(f, "Favicon({:?})", path.display())
            },
        }
    }
}

impl From<&'static str> for Target {
    fn from(text: &'static str) -> Self {
        Self::Text(Cow::Borrowed(text.as_bytes()))
    }
}

impl From<&'static [u8]> for Target {
    fn from(bytes: &'static [u8]) -> Self {
        Self::Bytes(Cow::Borrowed(bytes))
    }
}

impl From<String> for Target {
    fn from(text: String) -> Self {
        Self::Text(Cow::Owned(text.into_bytes()))
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
                util::content_type_from_ext(path)
            },
        }
    }
}
