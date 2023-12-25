use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::{Path, PathBuf};

use crate::util::get_extension;
use crate::{Body, Method, NetResult, Response};

/// Represents a server endpoint defined by an HTTP method and a URI path.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Route {
    NotFound,
    Shutdown,
    Get(String),
    Head(String),
    Post(String),
    Put(String),
    Patch(String),
    Delete(String),
    Trace(String),
    Options(String),
    Connect(String),
    Custom((String, String)),
}

impl Default for Route {
    fn default() -> Self {
        Self::Get(String::from("/"))
    }
}

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NotFound => write!(f, "NOT_FOUND"),
            Self::Shutdown => write!(f, "SHUTDOWN"),
            Self::Get(ref path) => write!(f, "GET {path}"),
            Self::Head(ref path) => write!(f, "HEAD {path}"),
            Self::Post(ref path) => write!(f, "POST {path}"),
            Self::Put(ref path) => write!(f, "PUT {path}"),
            Self::Patch(ref path) => write!(f, "PATCH {path}"),
            Self::Delete(ref path) => write!(f, "DELETE {path}"),
            Self::Trace(ref path) => write!(f, "TRACE {path}"),
            Self::Options(ref path) => write!(f, "OPTIONS {path}"),
            Self::Connect(ref path) => write!(f, "CONNECT {path}"),
            Self::Custom((ref method, ref path)) => {
                write!(f, "{method} {path}")
            },
        }
    }
}

impl Debug for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NotFound => write!(f, "NOT_FOUND"),
            Self::Shutdown => write!(f, "SHUTDOWN"),
            Self::Get(ref path) => write!(f, "GET({path:?})"),
            Self::Head(ref path) => write!(f, "HEAD({path:?})"),
            Self::Post(ref path) => write!(f, "POST({path:?})"),
            Self::Put(ref path) => write!(f, "PUT({path:?})"),
            Self::Patch(ref path) => write!(f, "PATCH({path:?})"),
            Self::Delete(ref path) => write!(f, "DELETE({path:?})"),
            Self::Trace(ref path) => write!(f, "TRACE({path:?})"),
            Self::Options(ref path) => write!(f, "OPTIONS({path:?})"),
            Self::Connect(ref path) => write!(f, "CONNECT({path:?})"),
            Self::Custom((ref method, ref path)) => {
                write!(f, "{method}({path:?})")
            },
        }
    }
}

impl Route {
    /// Returns a new `Route` based on the provided method and URI path.
    #[must_use]
    pub fn new(method: &Method, path: &str) -> Self {
        let path = path.to_string();

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
            Method::Custom(ref s) if s == "SHUTDOWN" => Self::Shutdown,
            Method::Custom(ref s) => Self::Custom((s.to_string(), path)),
        }
    }

    /// Returns this route's HTTP method.
    #[must_use]
    pub fn method(&self) -> Option<Method> {
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
            Self::Shutdown => Some(Method::Custom("SHUTDOWN".to_string())),
            Self::Custom((ref method, _)) => {
                Some(Method::Custom(method.to_string()))
            },
        }
    }

    /// Returns URI path component for this `Route`.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::NotFound | Self::Shutdown => None,
            Self::Get(ref path)
            | Self::Head(ref path)
            | Self::Post(ref path)
            | Self::Put(ref path)
            | Self::Patch(ref path)
            | Self::Delete(ref path)
            | Self::Trace(ref path)
            | Self::Options(ref path)
            | Self::Connect(ref path)
            | Self::Custom((_, ref path)) => Some(path),
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
    pub const fn is_shutdown(&self) -> bool {
        matches!(self, Self::Shutdown)
    }
}

/// The server router.
#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Router(pub BTreeMap<Route, Target>);

impl Display for Router {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_empty() {
            write!(f, "Router()")?;
        } else {
            writeln!(f, "Router(")?;

            for (route, target) in &self.0 {
                writeln!(f, "    {route:?} =>")?;
                writeln!(f, "        {target:?},")?;
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

    /// Appends the routes from `other` into this `Router`.
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
        match self.0.get(route).cloned() {
            // Allow HEAD requests for all configured GET routes.
            None if route.is_head() => route.path().map_or(
                Target::NotFound,
                |head_path| {
                    let get_route = Route::Get(head_path.to_string());
                    self.0.get(&get_route)
                        .cloned()
                        .unwrap_or(Target::NotFound)
                }),
            None => Target::NotFound,
            Some(target) => target,
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
        let mut res = match self.get_target(route) {
            // Route not found.
            Target::NotFound => {
                let target = self.get_not_found_target();
                Response::from_target(target, 404)?
            },
            // POST route found.
            target if route.is_post() => Response::from_target(target, 201)?,
            // Non-POST route found.
            target => Response::from_target(target, 200)?,
        };

        // Remove the response body for HEAD requests.
        if route.is_head() {
            res.body = Body::Empty;
        }

        Ok(res)
    }

    /// Configures handling of a GET request.
    #[must_use]
    pub fn get(mut self, uri_path: &str, file_path: &str) -> Self {
        self.insert_get(uri_path, file_path);
        self
    }

    /// Configures handling of a HEAD request.
    #[must_use]
    pub fn head(mut self, uri_path: &str, file_path: &str) -> Self {
        self.insert_head(uri_path, file_path);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post(mut self, uri_path: &str, file_path: &str) -> Self {
        self.insert_post(uri_path, file_path);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put(mut self, uri_path: &str, file_path: &str) -> Self {
        self.insert_put(uri_path, file_path);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch(mut self, uri_path: &str, file_path: &str) -> Self {
        self.insert_patch(uri_path, file_path);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete(mut self, uri_path: &str, file_path: &str) -> Self {
        self.insert_delete(uri_path, file_path);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace(mut self, uri_path: &str, file_path: &str) -> Self {
        self.insert_trace(uri_path, file_path);
        self
    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect(mut self,  uri_path: &str, file_path: &str) -> Self {
        self.insert_connect(uri_path, file_path);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options(mut self, uri_path: &str, file_path: &str) -> Self {
        self.insert_options(uri_path, file_path);
        self
    }

    /// Sets the static file path to a favicon icon.
    #[must_use]
    pub fn favicon(mut self, file_path: &str) -> Self {
        self.insert_favicon(file_path);
        self
    }

    /// Configures a file to return with 404 Not Found responses.
    #[must_use]
    pub fn not_found(mut self, file_path: &str) -> Self {
        self.insert_not_found(file_path);
        self
    }

    /// Configures handling of a GET request.
    pub fn insert_get(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Get(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures handling of a HEAD request.
    pub fn insert_head(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Head(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures handling of a POST request.
    pub fn insert_post(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Post(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures handling of a PUT request.
    pub fn insert_put(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Put(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures handling of a PATCH request.
    pub fn insert_patch(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Patch(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures handling of a DELETE request.
    pub fn insert_delete(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Delete(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures handling of a TRACE request.
    pub fn insert_trace(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Trace(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures handling of a CONNECT request.
    pub fn insert_connect(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Connect(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures handling of an OPTIONS request.
    pub fn insert_options(&mut self, uri_path: &str, file_path: &str) {
        let route = Route::Options(uri_path.to_string());
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Sets the static file path to a favicon icon.
    pub fn insert_favicon(&mut self, file_path: &str) {
        let route = Route::Get("/favicon.ico".to_string());
        let target = Target::Favicon(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Configures a file to return with 404 Not Found responses.
    pub fn insert_not_found(&mut self, file_path: &str) {
        let route = Route::NotFound;
        let target = Target::File(PathBuf::from(file_path));
        self.mount(route, target);
    }

    /// Returns a `RouteBuilder`.
    #[must_use]
    pub fn route(self, uri_path: &str) -> RouteBuilder {
        RouteBuilder::new(self, uri_path)
    }
}

/// A builder object for configuring one URI path to respond differently to
/// different HTTP methods.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteBuilder {
    pub router: Router,
    pub path: String,
}

impl RouteBuilder {
    /// Returns a new `RouteBuilder` instance.
    #[must_use]
    pub fn new(router: Router, path: &str) -> Self {
        let path = path.to_string();
        Self { router, path }
    }

    /// Configures handling of a GET request.
    #[must_use]
    pub fn get(mut self, target: Target) -> Self {
        let route = Route::Get(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a Head request.
    #[must_use]
    pub fn head(mut self, target: Target) -> Self {
        let route = Route::Head(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post(mut self, target: Target) -> Self {
        let route = Route::Post(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put(mut self, target: Target) -> Self {
        let route = Route::Put(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch(mut self, target: Target) -> Self {
        let route = Route::Patch(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete(mut self, target: Target) -> Self {
        let route = Route::Delete(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace(mut self, target: Target) -> Self {
        let route = Route::Trace(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options(mut self, target: Target) -> Self {
        let route = Route::Options(self.path.clone());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a CONNECT request.
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
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
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
