use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};
use std::path::Path;

use crate::{Body, Method, NetResult, Request, Response, Target, UriPath};

/// Represents a server end-point and the target resource to serve.
#[derive(Clone, Default)]
pub struct Route {
    pub method: Method,
    pub path: Option<UriPath>,
    pub target: Target,
}

impl Display for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:?}")
    }
}

impl Debug for Route {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self.path.as_ref() {
            Some(path) => write!(
                f,
                "{} {} -> {:?}",
                self.method.as_str(),
                path.as_str(),
                &self.target
            ),
            None if self.is_not_found() => {
                write!(f, "ANY -> {:?}", &self.target)
            },
            None if self.is_shutdown() => {
                write!(f, "SHUTDOWN -> {:?}", &self.target)
            },
            None => unreachable!(),
        }
    }
}

impl PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        self.method == other.method && self.path == other.path
    }
}

impl Eq for Route {}

impl PartialOrd for Route {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Route {
    fn cmp(&self, other: &Self) -> Ordering {
        let method_ordering = self.method.cmp(&other.method);

        if method_ordering != Ordering::Equal {
            return method_ordering;
        }

        self.path.cmp(&other.path)
    }
}

impl Hash for Route {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.method.hash(state);
        self.path.hash(state);
    }
}

impl Route {
    /// Returns a new `Route` based on the provided `Method`, `UriPath`, and
    /// `Target`.
    #[must_use]
    pub const fn new(method: Method, uri_path: UriPath, target: Target) -> Self {
        let path = Some(uri_path);
        Self { method, path, target }
    }

    /// Returns this route's HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns this route's URI path.
    #[must_use]
    pub const fn path(&self) -> Option<&UriPath> {
        self.path.as_ref()
    }

    /// Returns true if the `Route` is a GET route.
    #[must_use]
    pub const fn is_get(&self) -> bool {
        matches!(self.method, Method::Get)
    }

    /// Returns true if the `Route` is a HEAD route.
    #[must_use]
    pub const fn is_head(&self) -> bool {
        matches!(self.method, Method::Head)
    }

    /// Returns true if the `Route` is a POST route.
    #[must_use]
    pub const fn is_post(&self) -> bool {
        matches!(self.method, Method::Post)
    }

    /// Returns true if the `Route` is a PUT route.
    #[must_use]
    pub const fn is_put(&self) -> bool {
        matches!(self.method, Method::Put)
    }

    /// Returns true if the `Route` is a PATCH route.
    #[must_use]
    pub const fn is_patch(&self) -> bool {
        matches!(self.method, Method::Patch)
    }

    /// Returns true if the `Route` is a DELETE route.
    #[must_use]
    pub const fn is_delete(&self) -> bool {
        matches!(self.method, Method::Delete)
    }

    /// Returns true if the `Route` is a TRACE route.
    #[must_use]
    pub const fn is_trace(&self) -> bool {
        matches!(self.method, Method::Trace)
    }

    /// Returns true if the `Route` is a OPTIONS route.
    #[must_use]
    pub const fn is_options(&self) -> bool {
        matches!(self.method, Method::Options)
    }

    /// Returns true if the `Route` is a CONNECT route.
    #[must_use]
    pub const fn is_connect(&self) -> bool {
        matches!(self.method, Method::Connect)
    }

    /// Returns true if this `Route` is the server shutdown route.
    #[must_use]
    pub const fn is_shutdown(&self) -> bool {
        matches!(self.method, Method::Shutdown)
    }

    /// Returns true if this `Route` applies routes that are not found.
    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self.method, Method::Any)
    }
}

/// The server router.
#[derive(Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Router(pub BTreeSet<Route>);

impl Display for Router {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_empty() {
            write!(f, "Router()")
        } else {
            writeln!(f, "Router(")?;

            for route in &self.0 {
                writeln!(f, "    {route},")?;
            }

            write!(f, ")")
        }
    }
}

impl Debug for Router {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_empty() {
            write!(f, "Router()")
        } else {
            writeln!(f, "Router(")?;

            for route in &self.0 {
                writeln!(f, "    {route:?},")?;
            }

            write!(f, ")")
        }
    }
}

impl Router {
    /// Returns a new `Router` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if this `Router` contains the provided `Route`.
    #[must_use]
    pub fn contains(&self, route: &Route) -> bool {
        self.0.contains(route)
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

    /// Mount a new `Route` to the `Router`.
    pub fn mount(&mut self, route: Route) {
        self.0.insert(route);
    }

    /// Returns the `Target` for the given `Request` if a corresponding
    /// `Route` exists in this `Router`, or `Target::NotFound` if the route
    /// does not exist.
    #[must_use]
    pub fn get_target(&self, req: &Request) -> Target {
        let path = if matches!(req.method, Method::Any | Method::Shutdown) {
            None
        } else {
            Some(req.path.clone())
        };

        let mut query_route = Route {
            method: req.method,
            path,
            ..Route::default()
        };

        match self.0.get(&query_route) {
            // Route was found.
            Some(route) => route.target.clone(),
            // Allow HEAD requests for all configured GET routes.
            None if query_route.is_head() => {
                query_route.method = Method::Get;

                self.0.get(&query_route).map_or(
                    Target::NotFound,
                    |route| route.target.clone())
            },
            // Route was not found.
            None => Target::NotFound,
        }
    }

    /// Resolves the given `Request` into a `Response`.
    ///
    /// # Errors
    ///
    /// Returns an error if `ResponseBuilder::build` is unable to construct a
    /// `Response`.
    #[allow(clippy::similar_names)]
    pub fn resolve(&self, req: &Request) -> NetResult<Response> {
        let mut res = match self.get_target(req) {
            // Route not found.
            Target::NotFound => {
                let not_found_route = Route {
                    method: Method::Any,
                    path: None,
                    ..Route::default()
                };

                // Check for a configured "route not found" target.
                let target = self.0.get(&not_found_route).map_or(
                    Target::NotFound,
                    |route| route.target.clone());

                Response::builder().status_code(404).target(target).build()?
            },
            // POST route found.
            target if matches!(req.method, Method::Post) => {
                Response::builder().status_code(201).target(target).build()?
            },
            // Non-POST route found.
            target => {
                Response::builder().status_code(200).target(target).build()?
            },
        };

        // Remove the response body, if appropriate.
        if Body::should_be_empty(res.status.code(), &req.method) {
            res.body = Body::Empty;
        }

        Ok(res)
    }

    /// Configures a GET route that serves a file.
    #[must_use]
    pub fn get<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Get, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a HEAD route that serves a file.
    #[must_use]
    pub fn head<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Head, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a POST route that serves a file.
    #[must_use]
    pub fn post<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Post, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a PUT route that serves a file.
    #[must_use]
    pub fn put<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Put, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a PATCH route that serves a file.
    #[must_use]
    pub fn patch<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Patch, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a DELETE route that serves a file.
    #[must_use]
    pub fn delete<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Delete, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a TRACE route that serves a file.
    #[must_use]
    pub fn trace<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Trace, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures an OPTIONS route that serves a file.
    #[must_use]
    pub fn options<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Options, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a CONNECT route that serves a file.
    #[must_use]
    pub fn connect<P, F>(&mut self, uri_path: P, file_path: F) -> &mut Self
    where
        P: Into<UriPath>,
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = uri_path.into();
        let file_target = Target::File(file_path.into());
        let route = Route::new(Method::Connect, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a route that serves a favicon image file.
    #[must_use]
    pub fn favicon<F>(&mut self, file_path: F) -> &mut Self
    where
        F: Into<Cow<'static, Path>>,
    {
        let uri_path = "/favicon.ico".into();
        let file_target = Target::Favicon(file_path.into());
        let route = Route::new(Method::Get, uri_path, file_target);
        self.0.insert(route);
        self
    }

    /// Configures a file to be served in response to requests for routes that
    /// are not found.
    #[must_use]
    pub fn not_found<F>(&mut self, file_path: F) -> &mut Self
    where
        F: Into<Cow<'static, Path>>,
    {
        let route = Route {
            method: Method::Any,
            path: None,
            target: Target::File(file_path.into())
        };
        self.0.insert(route);
        self
    }

    /// Mount a shutdown `Route` to the `Router`.
    pub fn shutdown(&mut self) -> &mut Self {
        let route = Route {
            method: Method::Shutdown,
            path: None,
            target: Target::Shutdown
        };
        self.0.insert(route);
        self
    }

    /// Returns a `RouteBuilder`.
    #[must_use]
    pub fn route(&mut self, uri_path: &'static str) -> RouteBuilder {
        RouteBuilder::new(self.clone(), uri_path)
    }
}

/// Configures a single URI path to respond differently to different HTTP
/// methods.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct RouteBuilder {
    pub router: Router,
    pub uri_path: &'static str,
}

impl RouteBuilder {
    /// Returns a new `RouteBuilder` instance.
    #[must_use]
    pub const fn new(router: Router, uri_path: &'static str) -> Self {
        Self { router, uri_path }
    }

    /// Configures a GET route that serves the given `Target`.
    #[must_use]
    pub fn get<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Get, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Configures a HEAD route that serves the given `Target`.
    #[must_use]
    pub fn head<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Head, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Configures a POST route that serves the given `Target`.
    #[must_use]
    pub fn post<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Post, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Configures a PUT route that serves the given `Target`.
    #[must_use]
    pub fn put<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Put, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Configures a PATCH route that serves the given `Target`.
    #[must_use]
    pub fn patch<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Patch, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Configures a DELETE route that serves the given `Target`.
    #[must_use]
    pub fn delete<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Delete, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Configures a TRACE route that serves the given `Target`.
    #[must_use]
    pub fn trace<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Trace, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Configures an OPTIONS route that serves the given `Target`.
    #[must_use]
    pub fn options<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Options, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Configures a CONNECT route that serves the given `Target`.
    #[must_use]
    pub fn connect<T>(&mut self, target: T) -> &mut Self
    where
        T: Into<Target>,
    {
        let uri_path = self.uri_path.into();
        let target = target.into();
        let route = Route::new(Method::Connect, uri_path, target);
        self.router.mount(route);
        self
    }

    /// Returns the inner `Router` instance.
    #[must_use]
    pub fn apply(&mut self) -> Router {
        self.router.clone()
    }
}
