use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::{HeaderValue, Method, Request, Response};

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
    pub fn resolve(&self, route: &Route) -> Option<&Target> {
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

    /// Configures handling of a route.
    #[must_use]
    pub fn route(self, uri_path: &str) -> RouteBuilder {
        RouteBuilder::new(uri_path, self)
    }

    /// Configures handling of a GET request.
    #[must_use]
    pub fn get<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, uri_path);
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a GET request.
    #[must_use]
    pub fn get_with_handler<F>(mut self, uri_path: &str, handler: F) -> Self
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Get, uri_path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Post, uri_path);
        self.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Put, uri_path);
        self.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Patch, uri_path);
        self.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Delete, uri_path);
        self.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Trace, uri_path);
        self.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Connect, uri_path);
        self.mount(route, Target::Empty);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Options, uri_path);
        self.mount(route, Target::Empty);
        self
    }

    /// Sets the static file path to a favicon icon.
    #[must_use]
    pub fn favicon<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, "/favicon.ico");
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Sets the static file path to an HTML page returned by 404 responses.
    #[must_use]
    pub fn error_404<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, "__error");
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Returns the target resource for error 404 responses.
    #[must_use]
    pub fn error_handler(&self) -> &Target {
        let route = Route::Get("__error".to_string());
        self.resolve(&route).unwrap_or(&Target::Empty)
    }
}

pub struct RouteBuilder {
    path: String,
    router: Router,
}

impl RouteBuilder {
    /// Returns a new `RouteBuilder` instance.
    #[must_use]
    pub fn new(path: &str, router: Router) -> Self {
        Self {
            path: path.to_string(),
            router
        }
    }

    /// Configures handling of a GET request.
    #[must_use]
    pub fn get<F>(mut self, handler: F) -> Self
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Get, &self.path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post<F>(mut self, handler: F) -> Self 
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Post, &self.path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put<F>(mut self, handler: F) -> Self 
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Put, &self.path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch<F>(mut self, handler: F) -> Self 
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Patch, &self.path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete<F>(mut self, handler: F) -> Self 
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Delete, &self.path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace<F>(mut self, handler: F) -> Self 
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Trace, &self.path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options<F>(mut self, handler: F) -> Self 
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Options, &self.path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect<F>(mut self, handler: F) -> Self 
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Connect, &self.path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Returns the inner `Router`.
    #[must_use]
    pub fn apply(self) -> Router {
        self.router
    }
}

/// Target resources used by server end-points.
pub enum Target {
    Empty,
    Text(&'static str),
    Html(&'static str),
    Json(&'static str),
    Xml(&'static str),
    Bytes(Vec<u8>),
    File(PathBuf),
    Favicon(PathBuf),
    Fn(Box<dyn Fn(&Request, &Response) + Send + Sync>),
    FnMut(Arc<Mutex<dyn FnMut(&Request, &mut Response) + Send + Sync>>),
    FnOnce(Box<dyn FnOnce() + Send + Sync>),
}

impl Default for Target {
    fn default() -> Self {
        Self::Empty
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => Ok(()),
            Self::Text(ref s) => write!(f, "{s}"),
            Self::Html(ref s) => write!(f, "{s}"),
            Self::Json(ref s) => write!(f, "{s}"),
            Self::Xml(ref s) => write!(f, "{s}"),
            Self::Bytes(_) => write!(f, "Bytes {{ ... }}"),
            Self::File(_) => write!(f, "File {{ ... }}"),
            Self::Favicon(_) => write!(f, "Favicon {{ ... }}"),
            Self::Fn(_) => write!(f, "Fn {{ ... }}"),
            Self::FnMut(_) => write!(f, "FnMut {{ ... }}"),
            Self::FnOnce(_) => write!(f, "FnOnce {{ ... }}"),
        }
    }
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => f.debug_tuple("Empty").finish(),
            Self::Text(ref s) => f.debug_tuple("Text").field(s).finish(),
            Self::Html(ref s) => f.debug_tuple("Html").field(s).finish(),
            Self::Json(ref s) => f.debug_tuple("Json").field(s).finish(),
            Self::Xml(ref s) => f.debug_tuple("Xml").field(s).finish(),
            Self::Bytes(ref buf) => {
                f.debug_tuple("Bytes").field(buf).finish()
            },
            Self::File(ref path) => {
                f.debug_tuple("File").field(path).finish()
            },
            Self::Favicon(ref path) => {
                f.debug_tuple("Favicon").field(path).finish()
            },
            Self::Fn(_) => {
                f.debug_tuple("Fn closure").field(&"{ ... }").finish()
            },
            Self::FnMut(_) => {
                f.debug_tuple("FnMut closure").field(&"{ ... }").finish()
            },
            Self::FnOnce(_) => {
                f.debug_tuple("FnOnce closure").field(&"{ ... }").finish()
            },
        }
    }
}

impl PartialEq for Target {
    #[allow(clippy::match_like_matches_macro)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty)
                | (Self::Text(_), Self::Text(_))
                | (Self::Html(_), Self::Html(_))
                | (Self::Json(_), Self::Json(_))
                | (Self::Xml(_), Self::Xml(_))
                | (Self::Bytes(_), Self::Bytes(_))
                | (Self::File(_), Self::File(_))
                | (Self::Favicon(_), Self::Favicon(_))
                | (Self::Fn(_), Self::Fn(_))
                | (Self::FnMut(_), Self::FnMut(_))
                | (Self::FnOnce(_), Self::FnOnce(_)) => true,
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

    /// Returns true if the URI target type is a function.
    #[must_use]
    pub const fn is_function_handler(&self) -> bool {
        matches!(self, Self::Fn(_) | Self::FnMut(_) | Self::FnOnce(_))
    }

    /// Returns true if the URI target is a String.
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self,
            Self::Text(_) | Self::Html(_) | Self::Json(_) | Self::Xml(_))
    }

    /// Returns true if the URI target is a file path.
    #[must_use]
    pub const fn is_file_path(&self) -> bool {
        matches!(self, Self::File(_) | Self::Favicon(_))
    }

    /// Returns a Content-Type `HeaderValue` based on the `Target` variant.
    #[must_use]
    pub fn as_content_type(&self) -> Option<HeaderValue> {
        match self {
            Self::Text(_) => Some(b"text/plain; charset=utf-8"[..].into()),
            Self::Html(_) => Some(b"text/html; charset=utf-8"[..].into()),
            Self::Json(_) => Some(b"application/json"[..].into()),
            Self::Xml(_) => Some(b"application/xml"[..].into()),
            Self::Bytes(_) => Some(b"application/octet-stream"[..].into()),
            Self::File(ref path) => Some(HeaderValue::infer_content_type(path)),
            Self::Favicon(ref path) => Some(HeaderValue::infer_content_type(path)),
            _ => None,
        }
    }
}

/// A respresentation of the body content type.
#[derive(Clone, Hash)]
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
            Self::Text(ref s) => write!(f, "{s}"),
            Self::Html(ref s) => write!(f, "{s}"),
            Self::Json(ref s) => write!(f, "{s}"),
            Self::Xml(ref s) => write!(f, "{s}"),
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
            (Self::Text(_), Self::Text(_)) => true,
            (Self::Html(_), Self::Html(_)) => true,
            (Self::Json(_), Self::Json(_)) => true,
            (Self::Xml(_), Self::Xml(_)) => true,
            (Self::Image(_), Self::Image(_)) => true,
            (Self::Bytes(_), Self::Bytes(_)) => true,
            (Self::Favicon(_), Self::Favicon(_)) => true,
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


    /// Returns true if the URI target is a String.
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self,
            Self::Text(_) | Self::Html(_) | Self::Json(_) | Self::Xml(_))
    }

    /// Returns true if the URI target is a vector of bytes.
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Self::Image(_) | Self::Bytes(_) | Self::Favicon(_))
    }

    /// Returns a Content-Type `HeaderValue` based on the `Body` variant.
    #[must_use]
    pub fn as_content_type(&self) -> Option<HeaderValue> {
        match self {
            Self::Empty => None,
            Self::Text(_) => Some(b"text/plain; charset=utf-8"[..].into()),
            Self::Html(_) => Some(b"text/html; charset=utf-8"[..].into()),
            Self::Json(_) => Some(b"application/json"[..].into()),
            Self::Xml(_) => Some(b"application/xml"[..].into()),
            Self::Image(_) => Some(b"image"[..].into()),
            Self::Bytes(_) => Some(b"application/octet-stream"[..].into()),
            Self::Favicon(_) => Some(b"image/x-icon"[..].into()),
        }
    }

    /// Returns true if the body is safe/desireable to print to the terminal.
    #[must_use]
    pub const fn should_print(&self) -> bool {
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
        *self = Self::Text(text.to_string());
    }
}
