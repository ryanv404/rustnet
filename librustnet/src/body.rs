use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

/// A respresentation of the body content type.
#[derive(Clone, Hash, PartialOrd, Ord)]
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

impl Debug for Body {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => f.debug_tuple("Empty").finish(),
            Self::Text(ref s) => f.debug_tuple("Text").field(s).finish(),
            Self::Html(ref s) => f.debug_tuple("Html").field(s).finish(),
            Self::Json(ref s) => f.debug_tuple("Json").field(s).finish(),
            Self::Xml(ref s) => f.debug_tuple("Xml").field(s).finish(),
            Self::Image(_) => {
                f.debug_tuple("Image").field(&"{ ... }").finish()
            },
            Self::Bytes(_) => {
                f.debug_tuple("Bytes").field(&"{ ... }").finish()
            },
            Self::Favicon(_) => {
                f.debug_tuple("Favicon").field(&"{ ... }").finish()
            },
        }
    }
}

impl PartialEq for Body {
    #[allow(clippy::match_like_matches_macro)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Text(ref s1), Self::Text(ref s2)) => s1 == s2,
            (Self::Html(ref s1), Self::Html(ref s2)) => s1 == s2,
            (Self::Json(ref s1), Self::Json(ref s2)) => s1 == s2,
            (Self::Xml(ref s1), Self::Xml(ref s2)) => s1 == s2,
            (Self::Image(ref buf1), Self::Image(ref buf2)) => {
                &buf1[..] == &buf2[..]
            },
            (Self::Bytes(ref buf1), Self::Bytes(ref buf2)) => {
                &buf1[..] == &buf2[..]
            },
            (Self::Favicon(ref buf1), Self::Favicon(ref buf2)) => {
                &buf1[..] == &buf2[..]
            },
            _ => false,
        }
    }
}

impl Eq for Body {}

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
        match self {
            Self::Text(_)
                | Self::Html(_)
                | Self::Json(_) 
                | Self::Xml(_) => true,
            Self::Favicon(_)
                | Self::Bytes(_) 
                | Self::Image(_)
                | Self::Empty => false,
        }
    }

    /// Returns the body data as a bytes slice.
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

    /// Changes the body to a text type containing the provided string.
    pub fn text(&mut self, text: &str) {
        if !text.is_empty() {
            *self = Self::Text(text.to_string());
        }
    }
}
