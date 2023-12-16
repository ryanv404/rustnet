use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::{Path, PathBuf};

use crate::consts::CONTENT_TYPE;
use crate::{Header, HeaderValue, Method};

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
            (Self::Get(s1), Self::Get(s2)) => s1 == s2,
            (Self::Head(s1), Self::Head(s2)) => s1 == s2,
            (Self::Post(s1), Self::Post(s2)) => s1 == s2,
            (Self::Put(s1), Self::Put(s2)) => s1 == s2,
            (Self::Patch(s1), Self::Patch(s2)) => s1 == s2,
            (Self::Delete(s1), Self::Delete(s2)) => s1 == s2,
            (Self::Trace(s1), Self::Trace(s2)) => s1 == s2,
            (Self::Options(s1), Self::Options(s2)) => s1 == s2,
            (Self::Connect(s1), Self::Connect(s2)) => s1 == s2,
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
    /// Constructs a new route for the given HTTP method and URI path.
    #[must_use]
    pub fn new(method: Method, path: &str) -> Self {
        match method {
            Method::Get => Self::Get(path.to_string()),
            Method::Head => Self::Head(path.to_string()),
            Method::Post => Self::Post(path.to_string()),
            Method::Put => Self::Put(path.to_string()),
            Method::Patch => Self::Patch(path.to_string()),
            Method::Delete => Self::Delete(path.to_string()),
            Method::Trace => Self::Trace(path.to_string()),
            Method::Options => Self::Options(path.to_string()),
            Method::Connect => Self::Connect(path.to_string()),
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
    pub fn path(&self) -> &str {
        match self {
            Self::Get(path) => path.as_str(),
            Self::Head(path) => path.as_str(),
            Self::Post(path) => path.as_str(),
            Self::Put(path) => path.as_str(),
            Self::Patch(path) => path.as_str(),
            Self::Delete(path) => path.as_str(),
            Self::Trace(path) => path.as_str(),
            Self::Options(path) => path.as_str(),
            Self::Connect(path) => path.as_str(),
        }
    }

    /// Returns true if this `Route` is the server shutdown route.
    #[must_use]
    pub fn is_shutdown_route(&self) -> bool {
        matches!(self, Self::Delete(path) if path == "/__shutdown_server__")
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

    /// Configures handling of a GET request.
    #[must_use]
    pub fn get<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Get(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a HEAD request.
    #[must_use]
    pub fn head<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Head(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Post(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Put(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Patch(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Delete(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Trace(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Connect(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Options(uri_path.to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Sets the static file path to a favicon icon.
    #[must_use]
    pub fn favicon<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Get("/favicon.ico".to_string());
        let target = Target::Favicon(file_path.into());
        self.mount(route, target);
        self
    }

    /// Sets the static file path to an HTML page returned by 404 responses.
    #[must_use]
    pub fn error_404<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>,
    {
        let route = Route::Get("__error".to_string());
        let target = Target::File(file_path.into());
        self.mount(route, target);
        self
    }

    /// Sets the static file path to an HTML page returned by 404 responses.
    #[must_use]
    pub fn get_error_404(&self) -> Option<&Target> {
        self.resolve(&Route::Get("__error".to_string()))
    }

    /// Returns a `RouteBuilder` that is used to configure a single URI path to
    /// respond differently to different HTTP methods.
    #[must_use]
    pub const fn route(self, uri_path: &'static str) -> RouteBuilder {
        RouteBuilder::new(self, uri_path)
    }
}

/// A builder object for configuring one URI path to respond differently to
/// different HTTP methods.
#[derive(Debug, PartialEq, Eq)]
pub struct RouteBuilder {
    pub router: Router,
    pub path: &'static str,
}

impl RouteBuilder {
    /// Returns a new `RouteBuilder` instance.
    #[must_use]
    pub const fn new(router: Router, path: &'static str) -> Self {
        Self { router, path }
    }

    /// Configures handling of a GET request.
    #[must_use]
    pub fn get(mut self, target: Target) -> Self {
        let route = Route::Get(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a Head request.
    #[must_use]
    pub fn head(mut self, target: Target) -> Self {
        let route = Route::Head(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post(mut self, target: Target) -> Self {
        let route = Route::Post(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put(mut self, target: Target) -> Self {
        let route = Route::Put(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch(mut self, target: Target) -> Self {
        let route = Route::Patch(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete(mut self, target: Target) -> Self {
        let route = Route::Delete(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace(mut self, target: Target) -> Self {
        let route = Route::Trace(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options(mut self, target: Target) -> Self {
        let route = Route::Options(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect(mut self, target: Target) -> Self {
        let route = Route::Connect(self.path.to_string());
        self.router.mount(route, target);
        self
    }

    /// Returns the inner `Router` instance.
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
            Self::Html(s) => write!(f, "{s}"),
            Self::Json(s) => write!(f, "{s}"),
            Self::Xml(s) => write!(f, "{s}"),
            Self::Bytes(_) => write!(f, "Bytes {{ ... }}"),
            Self::File(_) => write!(f, "File {{ ... }}"),
            Self::Favicon(_) => write!(f, "Favicon {{ ... }}"),
        }
    }
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => f.debug_tuple("Empty").finish(),
            Self::Text(s) => f.debug_tuple("Text").field(s).finish(),
            Self::Html(s) => f.debug_tuple("Html").field(s).finish(),
            Self::Json(s) => f.debug_tuple("Json").field(s).finish(),
            Self::Xml(s) => f.debug_tuple("Xml").field(s).finish(),
            Self::Bytes(ref buf) => f.debug_tuple("Bytes").field(buf).finish(),
            Self::File(path) => f.debug_tuple("File").field(path).finish(),
            Self::Favicon(path) => f.debug_tuple("Favicon").field(path).finish(),
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
            (Self::Html(s1), Self::Html(s2)) => s1 == s2,
            (Self::Json(s1), Self::Json(s2)) => s1 == s2,
            (Self::Xml(s1), Self::Xml(s2)) => s1 == s2,
            (Self::Bytes(ref buf1), Self::Bytes(ref buf2)) => buf1[..] == buf2[..],
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

    /// Returns true if the URI target is a file.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
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
            }
            Self::Empty => unreachable!(),
        };

        Some(Header {
            name: CONTENT_TYPE,
            value,
        })
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
            },
        )
    }
}
