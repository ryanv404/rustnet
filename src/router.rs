use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::{Path, PathBuf};

use crate::{Body, Method, NetResult, Response, UriPath};
use crate::util::get_extension;

/// Represents a server endpoint defined by an HTTP method and a URI path.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Route {
    NotFound,
    Shutdown,
    Get(UriPath),
    Head(UriPath),
    Post(UriPath),
    Put(UriPath),
    Patch(UriPath),
    Delete(UriPath),
    Trace(UriPath),
    Options(UriPath),
    Connect(UriPath),
}

impl Default for Route {
    fn default() -> Self {
        Self::Get(UriPath::default())
    }
}

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NotFound => write!(f, "NOT FOUND"),
            Self::Shutdown => write!(f, "SHUTDOWN"),
            Self::Get(UriPath(ref path)) => write!(f, "GET {path}"),
            Self::Head(UriPath(ref path)) => write!(f, "HEAD {path}"),
            Self::Post(UriPath(ref path)) => write!(f, "POST {path}"),
            Self::Put(UriPath(ref path)) => write!(f, "PUT {path}"),
            Self::Patch(UriPath(ref path)) => write!(f, "PATCH {path}"),
            Self::Delete(UriPath(ref path)) => write!(f, "DELETE {path}"),
            Self::Trace(UriPath(ref path)) => write!(f, "TRACE {path}"),
            Self::Options(UriPath(ref path)) => write!(f, "OPTIONS {path}"),
            Self::Connect(UriPath(ref path)) => write!(f, "CONNECT {path}"),
        }
    }
}

impl Debug for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NotFound => write!(f, "NotFound"),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::Get(UriPath(ref path)) => write!(f, "Get({path:?})"),
            Self::Head(UriPath(ref path)) => write!(f, "Head({path:?})"),
            Self::Post(UriPath(ref path)) => write!(f, "Post({path:?})"),
            Self::Put(UriPath(ref path)) => write!(f, "Put({path:?})"),
            Self::Patch(UriPath(ref path)) => write!(f, "Patch({path:?})"),
            Self::Delete(UriPath(ref path)) => write!(f, "Delete({path:?})"),
            Self::Trace(UriPath(ref path)) => write!(f, "Trace({path:?})"),
            Self::Options(UriPath(ref path)) => write!(f, "Options({path:?})"),
            Self::Connect(UriPath(ref path)) => write!(f, "Connect({path:?})"),
        }
    }
}

impl Route {
    /// Returns a new `Route` based on the provided method and URI path.
    #[must_use]
    pub fn new(method: &Method, path: &str) -> Self {
        let path = path.into();

        match method {
            Method::Shutdown => Self::Shutdown,
            Method::Get => Self::Get(UriPath(path)),
            Method::Head => Self::Head(UriPath(path)),
            Method::Post => Self::Post(UriPath(path)),
            Method::Put => Self::Put(UriPath(path)),
            Method::Patch => Self::Patch(UriPath(path)),
            Method::Delete => Self::Delete(UriPath(path)),
            Method::Trace => Self::Trace(UriPath(path)),
            Method::Options => Self::Options(UriPath(path)),
            Method::Connect => Self::Connect(UriPath(path)),
        }
    }

    /// Returns this route's HTTP method.
    #[must_use]
    pub const fn method(&self) -> Option<Method> {
        match self {
            Self::NotFound => None,
            Self::Shutdown => Some(Method::Shutdown),
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
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::NotFound | Self::Shutdown => None,
            Self::Get(UriPath(ref path))
            | Self::Head(UriPath(ref path))
            | Self::Post(UriPath(ref path))
            | Self::Put(UriPath(ref path))
            | Self::Patch(UriPath(ref path))
            | Self::Delete(UriPath(ref path))
            | Self::Trace(UriPath(ref path))
            | Self::Options(UriPath(ref path))
            | Self::Connect(UriPath(ref path)) => Some(path),
        }
    }

    /// Returns true if this `Route` is the server shutdown route.
    #[must_use]
    pub const fn is_shutdown(&self) -> bool {
        matches!(self, Self::Shutdown)
    }

    /// Returns true if this `Route` is the 404 not found route.
    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound)
    }

    /// Returns true if the `Route` is a GET route.
    #[must_use]
    pub const fn is_get(&self) -> bool {
        matches!(self, Self::Get(_))
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

    /// Returns true if the `Route` is a PUT route.
    #[must_use]
    pub const fn is_put(&self) -> bool {
        matches!(self, Self::Put(_))
    }

    /// Returns true if the `Route` is a PATCH route.
    #[must_use]
    pub const fn is_patch(&self) -> bool {
        matches!(self, Self::Patch(_))
    }

    /// Returns true if the `Route` is a DELETE route.
    #[must_use]
    pub const fn is_delete(&self) -> bool {
        matches!(self, Self::Delete(_))
    }

    /// Returns true if the `Route` is a TRACE route.
    #[must_use]
    pub const fn is_trace(&self) -> bool {
        matches!(self, Self::Trace(_))
    }

    /// Returns true if the `Route` is a OPTIONS route.
    #[must_use]
    pub const fn is_options(&self) -> bool {
        matches!(self, Self::Options(_))
    }

    /// Returns true if the `Route` is a CONNECT route.
    #[must_use]
    pub const fn is_connect(&self) -> bool {
        matches!(self, Self::Connect(_))
    }
}

/// The server router.
#[derive(Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Router(pub BTreeMap<Route, Target>);

impl Display for Router {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_empty() {
            write!(f, "Router()")?;
        } else {
            writeln!(f, "Router(")?;

            for (route, target) in &self.0 {
                writeln!(f, "    {route:?} => {target:?},")?;
            }

            write!(f, ")")?;
        }

        Ok(())
    }
}

impl Debug for Router {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self}")
    }
}

impl Router {
    /// Returns a new `Router` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if there is an entry associated with the provided route.
    #[must_use]
    pub fn contains(&self, route: &Route) -> bool {
        self.0.contains_key(route)
    }

    /// Returns true if the `Router` contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Appends another `Router` collection to this one.
    pub fn append(&mut self, other: &mut Self) {
        self.0.append(&mut other.0);
    }

    /// Mount a new route to the `Router`.
    pub fn mount(&mut self, route: Route, target: Target) {
        self.0.insert(route, target);
    }

    /// Mount a shutdown route to the `Router`.
    pub fn mount_shutdown_route(&mut self) {
        let route = Route::Shutdown;
        let target = Target::Shutdown;
        self.0.insert(route, target);
    }

    /// Returns the `Target` for the given `Route` or `Target::NotFound` if
    /// the route does not exist in the `Router`.
    #[must_use]
    pub fn get_target(&self, route: &Route) -> Target {
        match self.0.get(route) {
            Some(target) => target.clone(),
            // Allow HEAD requests for all configured GET routes.
            None if route.is_head() => {
                route.path()
                    .map_or(Target::NotFound, |head_path| {
                        let get_route = Route::Get(head_path.into());
                        self.0.get(&get_route)
                            .cloned()
                            .unwrap_or(Target::NotFound)
                    })
            },
            None => Target::NotFound,
        }
    }

    /// Returns the `Target`, if configured, for requests to routes that do
    /// not exist.
    #[must_use]
    pub fn get_not_found_target(&self) -> Target {
        self.0.get(&Route::NotFound).cloned().unwrap_or(Target::Empty)
    }

    /// Resolves the given `Route` into a `Response`.
    ///
    /// # Errors
    ///
    /// Returns an error if `Response::from_target` is unable to construct a
    /// `Response` from the provided `Target` and status code.
    pub fn resolve(&self, route: &Route) -> NetResult<Response> {
        let (status_code, target) = match self.get_target(route) {
            // Route not found.
            Target::NotFound => (404, self.get_not_found_target()),
            // POST route found.
            target if route.is_post() => (201, target),
            // Non-POST route found.
            target => (200, target),
        };

        let mut res = Response::builder()
            .status_code(status_code)
            .target(target)
            .build()?;

        // Remove the response body for HEAD requests.
        if route.is_head() {
            res.body = Body::Empty;
        }

        Ok(res)
    }

    /// Configures a GET route that serves a file.
    #[must_use]
    pub fn get<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Get(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a HEAD route that serves a file.
    #[must_use]
    pub fn head<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Head(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a POST route that serves a file.
    #[must_use]
    pub fn post<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Post(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a PUT route that serves a file.
    #[must_use]
    pub fn put<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Put(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a PATCH route that serves a file.
    #[must_use]
    pub fn patch<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Patch(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a DELETE route that serves a file.
    #[must_use]
    pub fn delete<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Delete(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a TRACE route that serves a file.
    #[must_use]
    pub fn trace<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Trace(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures an OPTIONS route that serves a file.
    #[must_use]
    pub fn options<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Options(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a CONNECT route that serves a file.
    #[must_use]
    pub fn connect<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<PathBuf>,
        P: Into<UriPath>,
    {
        let route = Route::Connect(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a route that serves a favicon.
    #[must_use]
    pub fn favicon<F: Into<PathBuf>>(mut self, file: F) -> Self {
        let route = Route::Get("/favicon.ico".into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a route that serves a file for routes that are not
    /// found.
    #[must_use]
    pub fn not_found<F: Into<PathBuf>>(mut self, file: F) -> Self {
        let route = Route::NotFound;
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Returns a `RouteBuilder`.
    #[must_use]
    pub fn route(self, path: &str) -> RouteBuilder {
        RouteBuilder::new(self, path)
    }
}

/// Configures a single URI path to respond differently to different HTTP
// methods.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteBuilder {
    pub router: Router,
    pub path: UriPath,
}

impl RouteBuilder {
    /// Returns a new `RouteBuilder` instance.
    #[must_use]
    pub fn new(router: Router, path: &str) -> Self {
        let path = path.into();
        Self { router, path }
    }

    /// Configures a GET route that serves the given `target`.
    #[must_use]
    pub fn get(mut self, target: Target) -> Self {
        let route = Route::Get(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures a HEAD route that serves the given `target`.
    #[must_use]
    pub fn head(mut self, target: Target) -> Self {
        let route = Route::Head(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures a POST route that serves the given `target`.
    #[must_use]
    pub fn post(mut self, target: Target) -> Self {
        let route = Route::Post(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures a PUT route that serves the given `target`.
    #[must_use]
    pub fn put(mut self, target: Target) -> Self {
        let route = Route::Put(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures a PATCH route that serves the given `target`.
    #[must_use]
    pub fn patch(mut self, target: Target) -> Self {
        let route = Route::Patch(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures a DELETE route that serves the given `target`.
    #[must_use]
    pub fn delete(mut self, target: Target) -> Self {
        let route = Route::Delete(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures a TRACE route that serves the given `target`.
    #[must_use]
    pub fn trace(mut self, target: Target) -> Self {
        let route = Route::Trace(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures an OPTIONS route that serves the given `target`.
    #[must_use]
    pub fn options(mut self, target: Target) -> Self {
        let route = Route::Options(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures a CONNECT route that serves the given `target`.
    #[must_use]
    pub fn connect(mut self, target: Target) -> Self {
        let route = Route::Connect(self.path.clone());
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
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Empty,
    NotFound,
    Shutdown,
    Text(String),
    Html(String),
    Json(String),
    Xml(String),
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
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::NotFound => write!(f, "Not Found"),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::Bytes(_) => write!(f, "Bytes(...)"),
            Self::Text(ref s)
                | Self::Html(ref s)
                | Self::Json(ref s)
                | Self::Xml(ref s) => write!(f, "{s}"),
            Self::File(ref path) | Self::Favicon(ref path) => {
                write!(f, "{}", path.display())
            },
        }
    }
}

impl Debug for Target {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Empty => write!(f, "Empty"),
            Self::NotFound => write!(f, "NotFound"),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::Text(ref s) => write!(f, "Text({s:?})"),
            Self::Html(ref s) => write!(f, "Html({s:?})"),
            Self::Json(ref s) => write!(f, "Json({s:?})"),
            Self::Xml(ref s) => write!(f, "Xml({s:?})"),
            Self::Bytes(_) => write!(f, "Bytes(...)"),
            Self::File(ref path) => {
                write!(f, "File({:?})", path.display())
            },
            Self::Favicon(ref path) => {
                write!(f, "Favicon({:?})", path.display())
            },
        }
    }
}

impl From<&str> for Target {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<&[u8]> for Target {
    fn from(bytes: &[u8]) -> Self {
        Self::Bytes(bytes.to_vec())
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
            Self::Text(_) | Self::Shutdown => Some("text/plain; charset=utf-8"),
            Self::Html(_) => Some("text/html; charset=utf-8"),
            Self::Json(_) => Some("application/json"),
            Self::Xml(_) => Some("application/xml"),
            Self::Bytes(_) => Some("application/octet-stream"),
            Self::File(ref path) | Self::Favicon(ref path) => {
                Self::content_type_from_ext(path)
            },
        }
    }

    /// Returns a Content-Type header value from a file extension, if present.
    #[must_use]
    pub fn content_type_from_ext(path: &Path) -> Option<&str> {
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
