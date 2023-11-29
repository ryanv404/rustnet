use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::consts::CONTENT_TYPE;
use crate::{Header, HeaderValue, Method, Response};

/// Represents an endpoint defined by an HTTP method and a URI path.
#[derive(Debug, Ord, PartialOrd)]
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

impl PartialEq for Route {
    #[allow(clippy::match_same_arms)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Get(ref s1), Self::Get(ref s2)) => s1 == s2,
            (Self::Head(ref s1), Self::Head(ref s2)) => s1 == s2,
            (Self::Post(ref s1), Self::Post(ref s2)) => s1 == s2,
            (Self::Put(ref s1), Self::Put(ref s2)) => s1 == s2,
            (Self::Patch(ref s1), Self::Patch(ref s2)) => s1 == s2,
            (Self::Delete(ref s1), Self::Delete(ref s2)) => s1 == s2,
            (Self::Trace(ref s1), Self::Trace(ref s2)) => s1 == s2,
            (Self::Options(ref s1), Self::Options(ref s2)) => s1 == s2,
            (Self::Connect(ref s1), Self::Connect(ref s2)) => s1 == s2,
            _ => false,
        }
    }
}

impl Eq for Route {}

impl Default for Route {
    fn default() -> Self {
        Self::Get("/".to_string())
    }
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

    /// Returns this route's HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        match self {
            Self::Get(_) => Method::Get,
            Self::Head(_) => Method::Head,
            Self::Post(_) => Method::Post,
            Self::Put(_) => Method::Put,
            Self::Patch(_) => Method::Patch,
            Self::Delete(_) => Method::Delete,
            Self::Trace(_) => Method::Trace,
            Self::Options(_) => Method::Options,
            Self::Connect(_) => Method::Connect,
        }
    }

    /// Returns this route's URI path.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn path(&self) -> String {
        match self {
            Self::Get(ref path) => path.clone(),
            Self::Head(ref path) => path.clone(),
            Self::Post(ref path) => path.clone(),
            Self::Put(ref path) => path.clone(),
            Self::Patch(ref path) => path.clone(),
            Self::Delete(ref path) => path.clone(),
            Self::Trace(ref path) => path.clone(),
            Self::Options(ref path) => path.clone(),
            Self::Connect(ref path) => path.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Router(pub BTreeMap<Route, Target>);

impl PartialEq for Router {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for Router {}

impl Router {
    /// Returns a new `Router` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new route to the router.
    pub fn mount(&mut self, route: Route, target: Target) {
        self.0.insert(route, target);
    }

    /// Returns the configured `Target` for the route, if available.
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

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Post, uri_path);
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Put, uri_path);
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Patch, uri_path);
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Delete, uri_path);
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Trace, uri_path);
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Connect, uri_path);
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Options, uri_path);
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Sets the static file path to a favicon icon.
    #[must_use]
    pub fn favicon<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, "/favicon.ico");
        let target = Target::Favicon(file_path.into());
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

    /// Sets the static file path to an HTML page returned by 404 responses.
    #[must_use]
    pub fn get_error_404(&self) -> Option<&Target> {
        self.resolve(&Route::Get("__error".to_string()))
    }
}

/// A builder object for configuring one URI path to respond differently to
/// different HTTP methods.
#[derive(Debug, PartialEq, Eq)]
pub struct RouteBuilder {
    pub path: String,
    pub router: Router,
}

impl Default for RouteBuilder {
    fn default() -> Self {
        Self {
            path: String::new(),
            router: Router::new()
        }
    }
}

impl RouteBuilder {
    /// Returns a new `RouteBuilder` instance.
    #[must_use]
    pub fn new(path: &str, router: Router) -> Self {
        Self { path: path.to_string(), router }
    }

    /// Configures handling of a GET request.
    #[must_use]
    pub fn get<F>(mut self, handler: F) -> Self
    where
        F: FnMut(&mut Response) + Send + Sync + 'static
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
        F: FnMut(&mut Response) + Send + Sync + 'static
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
        F: FnMut(&mut Response) + Send + Sync + 'static
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
        F: FnMut(&mut Response) + Send + Sync + 'static
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
        F: FnMut(&mut Response) + Send + Sync + 'static
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
        F: FnMut(&mut Response) + Send + Sync + 'static
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
        F: FnMut(&mut Response) + Send + Sync + 'static
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
        F: FnMut(&mut Response) + Send + Sync + 'static
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

pub type FnHandler = dyn Fn(&Response) + Send + Sync + 'static;
pub type FnMutHandler = dyn FnMut(&mut Response) + Send + Sync + 'static;

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
    Fn(Arc<FnHandler>),
    FnMut(Arc<Mutex<FnMutHandler>>),
}

impl Default for Target {
    fn default() -> Self {
        Self::Empty
    }
}

impl Display for Target {
    #[allow(clippy::match_same_arms)]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => Ok(()),
            Self::Text(s) => write!(f, "{s}"),
            Self::Json(s) => write!(f, "{s}"),
            Self::Xml(s) => write!(f, "{s}"),
            Self::Bytes(_) => write!(f, "Bytes {{ ... }}"),
            Self::Html(_) => write!(f, "Html {{ ... }}"),
            Self::File(_) => write!(f, "File {{ ... }}"),
            Self::Favicon(_) => write!(f, "Favicon {{ ... }}"),
            Self::Fn(_) => write!(f, "Fn {{ ... }}"),
            Self::FnMut(_) => write!(f, "FnMut {{ ... }}"),
        }
    }
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => f.debug_tuple("Empty").finish(),
            Self::Text(s) => f.debug_tuple("Text").field(s).finish(),
            Self::Json(s) => f.debug_tuple("Json").field(s).finish(),
            Self::Xml(s) => f.debug_tuple("Xml").field(s).finish(),
            Self::Bytes(ref buf) => {
                f.debug_tuple("Bytes").field(buf).finish()
            },
            Self::Html(path) => {
                f.debug_tuple("Html").field(path).finish()
            },
            Self::File(path) => {
                f.debug_tuple("File").field(path).finish()
            },
            Self::Favicon(path) => {
                f.debug_tuple("Favicon").field(path).finish()
            },
            Self::Fn(_) => {
                f.debug_tuple("Fn").field(&"{ ... }").finish()
            },
            Self::FnMut(_) => {
                f.debug_tuple("FnMut").field(&"{ ... }").finish()
            },
        }
    }
}

impl PartialEq for Target {
    #[allow(clippy::match_same_arms)]
    #[allow(clippy::match_like_matches_macro)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Text(s1), Self::Text(s2)) => s1 == s2,
            (Self::Json(s1), Self::Json(s2)) => s1 == s2,
            (Self::Html(s1), Self::Html(s2)) => s1 == s2,
            (Self::Xml(s1), Self::Xml(s2)) => s1 == s2,
            (Self::Bytes(ref buf1), Self::Bytes(ref buf2)) => {
                buf1[..] == buf2[..]
            },
            (Self::File(ref p1), Self::File(ref p2)) => p1 == p2,
            (Self::Favicon(ref p1), Self::Favicon(ref p2)) => p1 == p2,
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
        matches!(self, Self::Fn(_) | Self::FnMut(_))
    }

    /// Returns true if the URI target is plain text.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns true if the URI target is JSON.
    #[must_use]
    pub const fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    /// Returns true if the URI target is HTML.
    #[must_use]
    pub const fn is_html(&self) -> bool {
        matches!(self, Self::Html(_))
    }

    /// Returns true if the URI target is a XML.
    #[must_use]
    pub const fn is_xml(&self) -> bool {
        matches!(self, Self::Xml(_))
    }

    /// Returns true if the URI target is a file path.
    #[must_use]
    pub const fn is_file_path(&self) -> bool {
        matches!(self, Self::File(_) | Self::Favicon(_))
    }

    /// Returns true if the URI target is a vector of bytes.
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    /// Returns a Content-Type `Header` based on the `Target` variant.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn as_content_type_header(&self) -> Option<Header> {
        if self.is_empty() {
            return None;
        }

        let value: HeaderValue = match self {
            Self::Text(_) => b"text/plain; charset=utf-8"[..].into(),
            Self::Html(_) => b"text/html; charset=utf-8"[..].into(),
            Self::Json(_) => b"application/json"[..].into(),
            Self::Xml(_) => b"application/xml"[..].into(),
            Self::Bytes(_) => b"application/octet-stream"[..].into(),
            Self::File(ref path) | Self::Favicon(ref path) => {
                Self::get_content_type_from_path(path)
            },
            Self::Fn(_) | Self::FnMut(_) => b"text/plain; charset=utf-8"[..].into(),
            Self::Empty => unreachable!(),
        };

        Some(Header { name: CONTENT_TYPE, value })
    }

    /// Infers a Content-Type header value based on the file extension.
    #[must_use]
    pub fn get_content_type_from_path(path: &Path) -> HeaderValue {
        path.extension().map_or_else(
            || b"application/octet-stream"[..].into(),
            |ext| match ext.to_str() {
                Some("html" | "htm") => b"text/html; charset=utf-8"[..].into(),
                Some("txt") => b"text/plain; charset=utf-8"[..].into(),
                Some("json") => b"application/json"[..].into(),
                Some("xml") => b"application/xml"[..].into(),
                Some("pdf") => b"application/pdf"[..].into(),
                Some("ico") => b"image/x-icon"[..].into(),
                Some("jpg" | "jpeg") => b"image/jpeg"[..].into(),
                Some("png") => b"image/png"[..].into(),
                Some("gif") => b"image/gif"[..].into(),
                _ => b"application/octet-stream"[..].into(),
            })
    }
}
