use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::Path;

use crate::util::get_extension;
use crate::Method;

/// Represents a server endpoint defined by an HTTP method and a URI path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Route {
    NotFound,
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

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NotFound => write!(f, "NotFound"),
            Self::Get(path) => write!(f, "GET {path}"),
            Self::Head(path) => write!(f, "HEAD {path}"),
            Self::Post(path) => write!(f, "POST {path}"),
            Self::Put(path) => write!(f, "PUT {path}"),
            Self::Patch(path) => write!(f, "PATCH {path}"),
            Self::Delete(path) => write!(f, "DELETE {path}"),
            Self::Trace(path) => write!(f, "TRACE {path}"),
            Self::Options(path) => write!(f, "OPTIONS {path}"),
            Self::Connect(path) => write!(f, "CONNECT {path}"),
        }
    }
}

impl Route {
    /// Returns a new `Route` based on the provided method and URI path.
    #[must_use]
    pub fn new(method: Method, path: &str) -> Self {
        let path = path.to_string();

        match method {
            Method::Get => Self::Get(path.into()),
            Method::Head => Self::Head(path.into()),
            Method::Post => Self::Post(path.into()),
            Method::Put => Self::Put(path.into()),
            Method::Patch => Self::Patch(path.into()),
            Method::Delete => Self::Delete(path.into()),
            Method::Trace => Self::Trace(path.into()),
            Method::Options => Self::Options(path.into()),
            Method::Connect => Self::Connect(path.into()),
        }
    }

    /// Returns this route's HTTP method.
    #[must_use]
    pub const fn method(&self) -> Option<Method> {
        match self {
            Self::NotFound => None,
            Self::Get(_) => Some(Method::Get),
            Self::Head(_) => Some(Method::Head),
            Self::Post(_) => Some(Method::Post),
            Self::Put(_) => Some(Method::Put),
            Self::Patch(_) => Some(Method::Patch),
            Self::Delete(_) => Some(Method::Delete),
            Self::Trace(_) => Some(Method::Trace),
            Self::Options(_) => Some(Method::Options),
            Self::Connect(_) => Some(Method::Connect),
        }
    }

    /// Returns URI path component for this `Route`.
    #[must_use]
    pub fn path(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotFound => None,
            Self::Get(path)
            | Self::Head(path)
            | Self::Post(path)
            | Self::Put(path)
            | Self::Patch(path)
            | Self::Delete(path)
            | Self::Trace(path)
            | Self::Options(path)
            | Self::Connect(path) => Some(path.clone()),
        }
    }

    /// Returns true if the `Route` is a HEAD route.
    #[must_use]
    pub const fn is_head(&self) -> bool {
        matches!(self, Self::Head(_))
    }

    /// Returns true if the `Route` is a POST route.
    #[must_use]
    pub const fn is_post(&self) -> bool {
        matches!(self, Self::Post(_))
    }

    /// Returns true if this `Route` is the server shutdown route.
    #[must_use]
    pub fn is_shutdown(&self) -> bool {
        self.path().map_or(false, |path| {
            path.eq_ignore_ascii_case("/__shutdown_server__")
        })
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

    /// Mount a new route to the router.
    pub fn mount(&mut self, route: Route, target: Target) {
        self.0.insert(route, target);
    }

    /// Mount a shutdown route.
    pub fn mount_shutdown_route(&mut self) {
        let route = Route::Delete("/__shutdown_server__".into());
        let target = Target::Shutdown;
        self.0.insert(route, target);
    }

    /// Returns the `Target` of the given `Route`, if present.
    #[must_use]
    pub fn get_target(&self, route: &Route) -> Target {
        self.0.get(route).copied().unwrap_or(Target::NotFound)
    }

    /// Returns the `Target` for non-existent routes.
    #[must_use]
    pub fn get_404_target(&self) -> Target {
        self.0
            .get(&Route::NotFound)
            .copied()
            .unwrap_or(Target::Empty)
    }

    /// Returns the `Target` and status code for the given `Route`.
    #[must_use]
    pub fn resolve(&self, route: &Route) -> (Target, u16) {
        let mut target = self.get_target(route);

        // Implement HEAD routes for all GET routes.
        if target.is_not_found() && route.is_head() {
            if let Route::Head(path) = route {
                let path = path.to_string();
                let get_route = Route::Get(path.into());
                let new_target = self.get_target(&get_route);

                if !new_target.is_not_found() {
                    target = new_target;
                }
            }
        }

        if target.is_not_found() {
            target = self.get_404_target();
            (target, 404)
        } else if route.is_post() {
            (target, 201)
        } else {
            (target, 200)
        }
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
    pub fn get(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
        let route = Route::Get(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a HEAD request.
    #[must_use]
    pub fn head(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
        let route = Route::Head(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
        let route = Route::Post(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
        let route = Route::Put(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
        let route = Route::Patch(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
        let route = Route::Delete(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
        let route = Route::Trace(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
        let route = Route::Connect(uri_path.into());
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options(
        mut self,
        uri_path: &'static str,
        file_path: &'static str,
    ) -> Self {
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

    /// Sets the file path to a static resource to return with 404 Not Found
    /// responses.
    #[must_use]
    pub fn not_found(mut self, file_path: &'static str) -> Self {
        let route = Route::NotFound;
        let target = Target::File(file_path.as_ref());
        self.mount(route, target);
        self
    }

    /// Returns a `RouteBuilder`.
    #[must_use]
    pub const fn route(self, uri_path: &'static str) -> RouteBuilder {
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

/// Target resources served by routes in a `Router`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Empty,
    NotFound,
    Shutdown,
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

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Target::Empty"),
            Self::NotFound => write!(f, "Target::NotFound"),
            Self::Shutdown => write!(f, "Target::Shutdown"),
            Self::Text(s) => write!(f, "Target::Text({s})"),
            Self::Html(s) => write!(f, "Target::Html({s})"),
            Self::Json(s) => write!(f, "Target::Json({s})"),
            Self::Xml(s) => write!(f, "Target::Xml({s})"),
            Self::File(_) => write!(f, "Target::File(...)"),
            Self::Bytes(_) => write!(f, "Target::Bytes(...)"),
            Self::Favicon(_) => write!(f, "Target::Favicon(...)"),
        }
    }
}

impl From<&'static str> for Target {
    fn from(text: &'static str) -> Self {
        Self::Text(text)
    }
}

impl From<&'static [u8]> for Target {
    fn from(bytes: &'static [u8]) -> Self {
        Self::Bytes(bytes)
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
    pub fn as_content_type(&self) -> Option<&'static str> {
        match self {
            Self::Empty | Self::NotFound => None,
            Self::Text(_) | Self::Shutdown => Some("text/plain; charset=utf-8"),
            Self::Html(_) => Some("text/html; charset=utf-8"),
            Self::Json(_) => Some("application/json"),
            Self::Xml(_) => Some("application/xml"),
            Self::Bytes(_) => Some("application/octet-stream"),
            Self::File(path) | Self::Favicon(path) => {
                Self::content_type_from_ext(path)
            },
        }
    }

    /// Returns a Content-Type header value from a file extension, if present.
    #[must_use]
    pub fn content_type_from_ext(path: &Path) -> Option<&'static str> {
        get_extension(path).and_then(|ext| match ext {
            "html" | "htm" => Some("text/html; charset=utf-8"),
            "txt" => Some("text/plain; charset=utf-8"),
            "json" => Some("application/json"),
            "xml" => Some("application/xml"),
            "pdf" => Some("application/pdf"),
            "ico" => Some("image/x-icon"),
            "jpg" | "jpeg" => Some("image/jpeg"),
            "png" => Some("image/png"),
            "gif" => Some("image/gif"),
            _ => None,
        })
    }
}
