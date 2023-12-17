use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::Path;

use crate::consts::CONTENT_TYPE;
use crate::{Header, HeaderValue, Method};

/// Represents an endpoint defined by an HTTP method and a URI path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Route {
    Get(Cow<'static, str>),
    Head(Cow<'static, str>),
    Post(Cow<'static, str>),
    Put(Cow<'static, str>),
    Patch(Cow<'static, str>),
    Delete(Cow<'static, str>),
    Trace(Cow<'static, str>),
    Options(Cow<'static, str>),
    Connect(Cow<'static, str>),
}

impl Default for Route {
    fn default() -> Self {
        Self::Get(Cow::Borrowed("/"))
    }
}

impl From<(Method, &str)> for Route {
    fn from((method, path): (Method, &str)) -> Self {
        let path = path.to_string();

        match method {
            Method::Get => Route::Get(path.into()),
            Method::Head => Route::Head(path.into()),
            Method::Post => Route::Post(path.into()),
            Method::Put => Route::Put(path.into()),
            Method::Patch => Route::Patch(path.into()),
            Method::Delete => Route::Delete(path.into()),
            Method::Trace => Route::Trace(path.into()),
            Method::Options => Route::Options(path.into()),
            Method::Connect => Route::Connect(path.into()),
        }
    }
}

impl Route {
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

    /// Returns URI path component for this `Route`.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn path(&self) -> Cow<'_, str> {
        match self {
            Self::Get(path) => path.clone(),
            Self::Head(path) => path.clone(),
            Self::Post(path) => path.clone(),
            Self::Put(path) => path.clone(),
            Self::Patch(path) => path.clone(),
            Self::Delete(path) => path.clone(),
            Self::Trace(path) => path.clone(),
            Self::Options(path) => path.clone(),
            Self::Connect(path) => path.clone(),
        }
    }

    /// Returns true if this `Route` is the server shutdown route.
    #[must_use]
    pub fn is_shutdown_route(&self) -> bool {
        matches!(self, Self::Delete(path)
            if path.eq_ignore_ascii_case("/__shutdown_server__"))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Router(pub BTreeMap<Route, Target>);

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
    pub fn resolve(&self, route: &Route) -> Option<Target> {
        self.0.get(route).cloned()
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
    pub fn get(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Get(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a HEAD request.
    #[must_use]
    pub fn head(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Head(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Post(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Put(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Patch(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Delete(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Trace(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Connect(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options(mut self, uri_path: &'static str, file_path: &'static str) -> Self {
        let route = Route::Options(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Sets the static file path to a favicon icon.
    #[must_use]
    pub fn favicon(mut self, file_path: &'static str) -> Self {
        let route = Route::Get("/favicon.ico".into());
        let target = Target::Favicon(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Sets the static file path to an HTML page returned by 404 responses.
    #[must_use]
    pub fn error_404(mut self, file_path: &'static str) -> Self {
        let route = Route::Get("__error".into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Sets the static file path to an HTML page returned by 404 responses.
    #[must_use]
    pub fn get_error_404(&self) -> Option<Target> {
        self.resolve(&Route::Get(Cow::Borrowed("__error")))
    }

    /// Returns a `RouteBuilder` that is used to configure a single URI path to
    /// respond differently to different HTTP methods.
    #[must_use]
    pub fn route(self, uri_path: &'static str) -> RouteBuilder {
        RouteBuilder::new(self, uri_path)
    }
}

/// A builder object for configuring one URI path to respond differently to
/// different HTTP methods.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
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
        let route = Route::Get(self.path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a Head request.
    #[must_use]
    pub fn head(mut self, target: Target) -> Self {
        let route = Route::Head(self.path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post(mut self, target: Target) -> Self {
        let route = Route::Post(self.path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put(mut self, target: Target) -> Self {
        let route = Route::Put(self.path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch(mut self, target: Target) -> Self {
        let route = Route::Patch(self.path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete(mut self, target: Target) -> Self {
        let route = Route::Delete(self.path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace(mut self, target: Target) -> Self {
        let route = Route::Trace(self.path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options(mut self, target: Target) -> Self {
        let route = Route::Options(self.path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect(mut self, target: Target) -> Self {
        let route = Route::Connect(self.path.into());
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Empty,
    Text(&'static str),
    Html(&'static str),
    Json(&'static str),
    Xml(&'static str),
    Bytes(&'static [u8]),
    File(&'static Path),
    Favicon(&'static Path),
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
            Self::File(path) | Self::Favicon(path) => {
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
