use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::consts::CONTENT_TYPE;
use crate::{
    HeaderName, HeaderValue, Method, NetError, NetReader, NetResult,
    ParseErrorKind, Request, Response,
};

/// Represents an endpoint defined by an HTTP method and a URI path.
#[derive(Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub enum Route {
    Get(String),
    Head(String),
    Post(String),
    Put(String),
    Patch(String),
    Delete(String),
    Trace(String),
    Options(String),
    Connect(String),
}

impl Route {
    /// Constructs a new `Route` instance.
    #[must_use]
    pub fn new(method: Method, uri_path: &str) -> Self {
        let path = uri_path.to_string();

        match method {
            Method::Get => Self::Get(path),
            Method::Head => Self::Head(path),
            Method::Post => Self::Post(path),
            Method::Put => Self::Put(path),
            Method::Patch => Self::Patch(path),
            Method::Delete => Self::Delete(path),
            Method::Trace => Self::Trace(path),
            Method::Options => Self::Options(path),
            Method::Connect => Self::Connect(path),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Router(pub BTreeMap<Route, Target>);

impl Router {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mount(&mut self, route: Route, target: Target) {
        self.0.insert(route, target);
    }

    #[must_use]
    pub fn get(&self, route: &Route) -> Option<&Target> {
        self.0.get(route)
    }

    /// Returns true if there is an entry associated with `Route`.
    #[must_use]
    pub fn contains(&self, route: &Route) -> bool {
        self.0.contains_key(route)
    }

    /// Returns true if the `Router` contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the target resource for error 404 responses.
    #[must_use]
    pub fn error_handler(&self) -> &Target {
        let route = Route::Get("__error".to_string());
        self.get(&route).unwrap_or(&Target::Empty)
    }

    /// Resolves a `Request` into a `Response` based on the provided `Router`.
    pub fn resolve(
        req: &mut Request,
        router: &Arc<Self>,
    ) -> NetResult<Response> {
        if router.is_empty() {
            let mut target = Target::Text("This server has no routes configured.");
            return Response::new(502, &mut target, req);
        }

        let method = req.method();
        let mut maybe_target = router.get(&req.route());

        match (maybe_target.as_mut(), method) {
            (Some(target), Method::Get) => {
                Response::new(200, target, req)
            },
            (Some(target), Method::Head) => {
                Response::new(200, target, req)
            },
            (Some(target), Method::Post) => {
                Response::new(200, target, req)
            },
            (Some(target), Method::Put) => {
                Response::new(200, target, req)
            },
            (Some(target), Method::Patch) => {
                Response::new(200, target, req)
            },
            (Some(target), Method::Delete) => {
                Response::new(200, target, req)
            },
            (Some(target), Method::Trace) => {
                Response::new(200, target, req)
            },
            (Some(target), Method::Options) => {
                Response::new(200, target, req)
            },
            (Some(target), Method::Connect) => {
                Response::new(200, target, req)
            },
            (None, Method::Head) => {
                // Handle a HEAD request for a route that does not exist
                // but does exist as for a GET request.
                let route = Route::Get(req.request_line.path.clone());

                let (code, mut target) = router.get(&route).map_or_else(
                    || {
                        // No route exists for a GET request either.
                        (404, router.error_handler())
                    },
                    |target| {
                        // GET route exists so send it as a HEAD response.
                        (200, target)
                    }
                );

                Response::new(code, &mut target, req)
            },
            (None, _) => {
                // Handle routes that do not exist.
                Response::new(404, &mut router.error_handler(), req)
            },
        }
    }
}

/// Target resources used by server end-points.
pub enum Target {
    Empty,
    File(PathBuf),
    Favicon(PathBuf),
    Html(&'static str),
    Text(&'static str),
    Json(&'static str),
    Xml(&'static str),
    Fn(Box<dyn Fn(&Request, &Response) + Send + Sync>),
    FnMut(Arc<Mutex<dyn FnMut(&Request, &mut Response) + Send + Sync>>),
    FnOnce(Box<dyn FnOnce() -> Body + Send + Sync>),
}

impl Default for Target {
    fn default() -> Self {
        Self::Empty
    }
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Target::Empty"),
            Self::File(ref path) => write!(f, "Target::File({})", path.display()),
            Self::Favicon(ref path) => write!(f, "Target::Favicon({})", path.display()),
            Self::Html(s) => write!(f, "Target::Html({s})"),
            Self::Text(s) => write!(f, "Target::Text({s})"),
            Self::Json(s) => write!(f, "Target::Json({s})"),
            Self::Xml(s) => write!(f, "Target::Xml({s})"),
            Self::Fn(_) => write!(f, "Target::Fn(Fn(&Request, &Response))"),
            Self::FnMut(_) => write!(f, "Target::FnMut(FnMut(&Request, &mut Response))"),
            Self::FnOnce(_) => write!(f, "Target::FnOnce(FnOnce() -> Body)"),
        }
    }
}

impl PartialEq for Target {
    #[allow(clippy::match_like_matches_macro)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::File(_), Self::File(_)) => true,
            (Self::Html(_), Self::Html(_)) => true,
            (Self::Favicon(_), Self::Favicon(_)) => true,
            (Self::Text(_), Self::Text(_)) => true,
            (Self::Json(_), Self::Json(_)) => true,
            (Self::Xml(_), Self::Xml(_)) => true,
            (Self::Fn(_), Self::Fn(_)) => true,
            (Self::FnMut(_), Self::FnMut(_)) => true,
            (Self::FnOnce(_), Self::FnOnce(_)) => true,
            _ => false,
        }
    }
}

impl Eq for Target {}

impl Target {
    /// Returns a default `Target` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the URI target type is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns true if the URI target type is text.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns true if the URI target type is a file.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }

    /// Returns true if the URI target type is HTML.
    #[must_use]
    pub const fn is_html(&self) -> bool {
        matches!(self, Self::Html(_))
    }

    /// Returns true if the URI target type is JSON.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns true if the URI target type is XML.
    #[must_use]
    pub const fn is_xml(&self) -> bool {
        matches!(self, Self::Xml(_))
    }

    /// Returns true if the URI target type is handler function.
    #[must_use]
    pub const fn is_handler(&self) -> bool {
        matches!(self, Self::Fn(_) | Self::FnMut(_) | Self::FnOnce(_))
    }
}

/// A respresentation of the body content type.
#[derive(Clone, Debug, Hash)]
pub enum Body {
    Empty,
    Text(String),
    Html(String),
    Json(String),
    Xml(String),
    File(PathBuf),
    Favicon(Vec<u8>),
    Bytes(Vec<u8>),
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
            Self::Text(ref s) => write!(f, "{s}"),
            Self::Html(ref s) => write!(f, "{s}"),
            Self::Json(ref s) => write!(f, "{s}"),
            Self::Xml(ref s) => write!(f, "{s}"),
            Self::File(_) => Ok(()),
            Self::Favicon(_) => Ok(()),
            Self::Bytes(_) => Ok(()),
        }
    }
}

impl PartialEq for Body {
    #[allow(clippy::match_like_matches_macro)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Text(_), Self::Text(_)) => true,
            (Self::Html(_), Self::Html(_)) => true,
            (Self::Json(_), Self::Json(_)) => true,
            (Self::Xml(_), Self::Xml(_)) => true,
            (Self::File(_), Self::File(_)) => true,
            (Self::Favicon(_), Self::Favicon(_)) => true,
            (Self::Bytes(_), Self::Bytes(_)) => true,
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

    /// Returns true if the body is bytes.
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    /// Returns true if the body type is text.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns true if the body type is HTML.
    #[must_use]
    pub const fn is_html(&self) -> bool {
        matches!(self, Self::Html(_))
    }

    /// Returns true if the body type is JSON.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns true if the body type is XML.
    #[must_use]
    pub const fn is_xml(&self) -> bool {
        matches!(self, Self::Xml(_))
    }

    /// Returns true if the body is a path to a local file.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }

    /// Returns true if the body is a favicon.
    #[must_use]
    pub const fn is_favicon(&self) -> bool {
        matches!(self, Self::Favicon(_))
    }

    /// Returns a Content-Type `HeaderName`and `HeaderValue` based on the
    /// `Body` variant.
    #[must_use]
    pub fn as_content_type(&self) -> Option<(HeaderName, HeaderValue)> {
        let value: HeaderValue = match self {
            Self::Empty => return None,
            Self::Text(_) => b"text/plain; charset=utf-8"[..].into(),
            Self::Html(_) => b"text/html; charset=utf-8"[..].into(),
            Self::Json(_) => b"application/json"[..].into(),
            Self::Xml(_) => b"application/xml"[..].into(),
            Self::File(_) => return None,
            Self::Favicon(_) => b"image/x-icon"[..].into(),
            Self::Bytes(_) => return None,
        };

        Some((CONTENT_TYPE, value))
    }

    /// Returns true if the body is safe to print to the terminal.
    #[must_use]
    pub const fn is_printable(&self) -> bool {
        match self {
            Self::Text(_) | Self::Html(_) | Self::Json(_) 
                | Self::Xml(_) => true,
            Self::Empty | Self::Favicon(_) | Self::Bytes(_) 
                | Self::File(_) => false,
        }
    }

    /// Uses header values to read and parse the message body.
    #[must_use]
    pub fn parse(
        reader: &mut NetReader,
        len_val: usize,
        type_val: &str
    ) -> NetResult<Body> {
        let Ok(num_bytes) = u64::try_from(len_val) else {
            return Err(ParseErrorKind::Body.into());
        };

        let mut buf = Vec::with_capacity(len_val);
        let mut rdr = reader.take(num_bytes);

        // TODO: handle chunked data and partial reads.
        match rdr.read_to_end(&mut buf) {
            Ok(_) => {
                let body = match type_val {
                    s if s.is_empty() => Self::Empty,
                    s if s.contains("text/plain") => {
                        let str = String::from_utf8_lossy(&buf).to_string();
                        Self::Text(str)
                    },
                    s if s.contains("text/html") => {
                        let str = String::from_utf8_lossy(&buf).to_string();
                        Self::Html(str)
                    },
                    s if s.contains("application/json") => {
                        let str = String::from_utf8_lossy(&buf).to_string();
                        Self::Json(str)
                    },
                    s if s.contains("application/xml") => {
                        let str = String::from_utf8_lossy(&buf).to_string();
                        Self::Xml(str)
                    },
                    s if s.contains("image/x-icon") => {
                        Self::Favicon(buf)
                    },
                    _ => Self::Bytes(buf),
                };

                Ok(body)
            },
            Err(e) => Err(NetError::ReadError(e.kind())),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Empty => &b""[..],
            Self::Text(ref s) => s.as_bytes(),
            Self::Html(ref s) => s.as_bytes(),
            Self::Json(ref s) => s.as_bytes(),
            Self::Xml(ref s) => s.as_bytes(),
            Self::File(_) => &b""[..],
            Self::Favicon(ref buf) => buf.as_slice(),
            Self::Bytes(ref buf) => buf.as_slice(),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Text(ref s) => s.len(),
            Self::Html(ref s) => s.len(),
            Self::Json(ref s) => s.len(),
            Self::Xml(ref s) => s.len(),
            Self::File(_) => 0,
            Self::Favicon(ref buf) => buf.len(),
            Self::Bytes(ref buf) => buf.len(),
        }
    }
}
