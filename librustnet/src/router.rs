use std::path::PathBuf;

use crate::{Method, Request, RoutesMap, Status};

/// Represents an endpoint defined by an HTTP method and a URI path.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Route {
    pub method: Method,
    pub uri_path: String,
}

impl Route {
    /// Constructs a new `Route` instance.
    #[must_use]
    pub fn new(method: Method, uri_path: &str) -> Self {
        let uri_path = uri_path.to_string();
        Self { method, uri_path }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Router {
    pub routes: RoutesMap,
}

impl Router {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mount(&mut self, route: Route, target: Target) {
        self.routes.insert(route, target);
    }

    #[must_use]
    pub fn get_target(&self, route: &Route) -> Option<&Target> {
        self.routes.get(route)
    }

    #[must_use]
    pub fn route_exists(&self, route: &Route) -> bool {
        self.routes.contains_key(route)
    }

    #[must_use]
    pub fn get_error_page(&self) -> Target {
        let route = Route::new(Method::Get, "__error");
        self.get_target(&route).cloned().unwrap_or_default()
    }

    #[must_use]
    pub fn resolve(&self, req: &Request) -> Resolved {
        let method = req.method();

        match self.get_target(&req.route()) {
            Some(target) => {
                req.log_status(200);
                Resolved::new(Status(200), method, target)
            },
            None if method == Method::Head => {
                // Handle a HEAD request for a route that does not exist
                // but does exist as for a GET request.
                let route = Route::new(Method::Get, req.path());
                
                self.get_target(&route).map_or_else(
                    || {
                        // No route exists for a GET request either.
                        req.log_status(404);
                        Resolved::new(Status(404), Method::Head, &self.get_error_page())
                    },
                    |target| {
                        // GET route exists so send it as a HEAD response.
                        req.log_status(200);
                        Resolved::new(Status(200), Method::Head, target)
                    })
            },
            None => {
                // Handle routes that do not exist.
                req.log_status(404);
                Resolved::new(Status(404), method, &self.get_error_page())
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Empty,
    File(PathBuf),
    Text(String),
    Bytes(Vec<u8>),
}

impl Default for Target {
    fn default() -> Self {
        Self::Empty
    }
}

impl Target {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(*self, Self::Text(_))
    }

    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(*self, Self::File(_))
    }

    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(*self, Self::Bytes(_))
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::Empty
    }
}

// TODO: Just construct a `Response` directly instead of using a `Resolved` object.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub status: Status,
    pub method: Method,
    pub target: Target,
}

impl Resolved {
    #[must_use]
    pub fn new(status: Status, method: Method, target: &Target) -> Self {
        let target = target.to_owned();
        Self { status, method, target }
    }

    /// Returns the response status.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Returns the HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the resolved target resource.
    #[must_use]
    pub const fn target(&self) -> &Target {
        &self.target
    }
}
