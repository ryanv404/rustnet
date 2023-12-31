use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::Path;

use crate::{Body, Method, NetResult, Response, Target, UriPath};

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
    pub fn new(method: &Method, path: String) -> Self {
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
    pub fn not_found_target(&self) -> Target {
        self.0.get(&Route::NotFound).cloned().unwrap_or(Target::Empty)
    }

    /// Resolves the given `Route` into a `Response`.
    ///
    /// # Errors
    ///
    /// Returns an error if `ResponseBuilder::build` is unable to construct the
    /// `Response`.
    pub fn resolve(&self, route: &Route) -> NetResult<Response> {
        let mut res = match self.get_target(route) {
            // Route not found.
            Target::NotFound => {
                let target = self.not_found_target();
                Response::builder().status_code(404).target(target).build()?
            },
            // POST route found.
            target if route.is_post() => {
                Response::builder().status_code(201).target(target).build()?
            },
            // Non-POST route found.
            target => {
                Response::builder().status_code(200).target(target).build()?
            },
        };

        // Remove the body if appropriate.
        if let Some(method) = route.method() {
            if Body::should_be_empty(res.status_code(), method) {
                res.body = Body::Empty;
            }
        }

        Ok(res)
    }

    /// Configures a GET route that serves a file.
    #[must_use]
    pub fn get<F, P>(mut self, path: P, file: F) -> Self
    where
        F: Into<Cow<'static, Path>>,
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
        F: Into<Cow<'static, Path>>,
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
        F: Into<Cow<'static, Path>>,
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
        F: Into<Cow<'static, Path>>,
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
        F: Into<Cow<'static, Path>>,
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
        F: Into<Cow<'static, Path>>,
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
        F: Into<Cow<'static, Path>>,
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
        F: Into<Cow<'static, Path>>,
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
        F: Into<Cow<'static, Path>>,
        P: Into<UriPath>,
    {
        let route = Route::Connect(path.into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a route that serves a favicon.
    #[must_use]
    pub fn favicon<F: Into<Cow<'static, Path>>>(mut self, file: F) -> Self {
        let route = Route::Get("/favicon.ico".into());
        let target = Target::File(file.into());
        self.0.insert(route, target);
        self
    }

    /// Configures a route that serves a file for routes that are not
    /// found.
    #[must_use]
    pub fn not_found<F: Into<Cow<'static, Path>>>(mut self, file: F) -> Self {
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
    pub fn get<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Get(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Configures a HEAD route that serves the given `target`.
    #[must_use]
    pub fn head<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Head(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Configures a POST route that serves the given `target`.
    #[must_use]
    pub fn post<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Post(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Configures a PUT route that serves the given `target`.
    #[must_use]
    pub fn put<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Put(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Configures a PATCH route that serves the given `target`.
    #[must_use]
    pub fn patch<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Patch(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Configures a DELETE route that serves the given `target`.
    #[must_use]
    pub fn delete<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Delete(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Configures a TRACE route that serves the given `target`.
    #[must_use]
    pub fn trace<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Trace(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Configures an OPTIONS route that serves the given `target`.
    #[must_use]
    pub fn options<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Options(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Configures a CONNECT route that serves the given `target`.
    #[must_use]
    pub fn connect<T: Into<Target>>(mut self, target: T) -> Self {
        let route = Route::Connect(self.path.clone());
        self.router.mount(route, target.into());
        self
    }

    /// Returns the inner `Router` instance.
    #[must_use]
    pub fn apply(self) -> Router {
        self.router
    }
}
