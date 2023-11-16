use std::path::PathBuf;

use crate::{Method, Request, RoutesMap, Status};

/// Represents an endpoint defined by an HTTP method and a URI.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Route {
    pub method: Method,
    pub uri: String,
}

impl Route {
    /// Constructs a `Route` instance from an HTTP method and a URI.
    #[must_use]
    pub fn new(method: Method, uri: &str) -> Self {
        let uri = uri.to_string();
        Self { method, uri }
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

    pub fn mount(&mut self, route: Route) {
        self.routes.insert(route, PathBuf::new());
    }

    pub fn mount_with_path<P: Into<PathBuf>>(&mut self, route: Route, path: P) {
        self.routes.insert(route, path.into());
    }

    #[must_use]
    pub fn get_error_page(&self) -> Option<PathBuf> {
        let route = Route::new(Method::Get, "__error");
        self.routes.get(&route).cloned()
    }

    #[must_use]
    pub fn resolve(&self, req: &Request) -> Resolved {
        let req_route = req.route();
        let req_method = req_route.method;

        let maybe_route = self.routes.get(&req_route);
        let route_exists = maybe_route.is_some();

        match req_method {
            // Handle a GET route that exists.
            Method::Get if route_exists => {
                let maybe_path = maybe_route.unwrap();
                let maybe_path = if maybe_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(maybe_path.clone())
                };

                req.log(200);
                Resolved::new(Status(200), req_method, maybe_path)
            },
            // Implement the HEAD method for all GET routes.
            Method::Head => {
                // Check if the route exists as a GET route.
                let mut req_route = req_route;
                req_route.method = Method::Get;

                if let Some(path) = self.routes.get(&req_route) {
                    // Handle a HEAD request for a GET route that exists.
                    let maybe_path = if path.as_os_str().is_empty() {
                        None
                    } else {
                        Some(path.clone())
                    };

                    req.log(200);
                    Resolved::new(Status(200), Method::Head, maybe_path)
                } else {
                    // Handle HEAD request for GET routes that do not exist.
                    req.log(404);
                    Resolved::new(Status(404), Method::Head, self.get_error_page())
                }
            },
            // Handle a POST route that exists.
            Method::Post if route_exists => {
                let maybe_path = maybe_route.unwrap();
                let maybe_path = if maybe_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(maybe_path.clone())
                };

                req.log(201);
                Resolved::new(Status(201), req_method, maybe_path)
            },
            // Handle a PUT route that exists.
            Method::Put if route_exists => {
                let maybe_path = maybe_route.unwrap();
                let maybe_path = if maybe_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(maybe_path.clone())
                };

                req.log(200);
                Resolved::new(Status(200), req_method, maybe_path)
            },
            // Handle a PATCH route that exists.
            Method::Patch if route_exists => {
                let maybe_path = maybe_route.unwrap();
                let maybe_path = if maybe_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(maybe_path.clone())
                };

                req.log(200);
                Resolved::new(Status(200), req_method, maybe_path)
            },
            // Handle a DELETE route that exists.
            Method::Delete if route_exists => {
                let maybe_path = maybe_route.unwrap();
                let maybe_path = if maybe_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(maybe_path.clone())
                };

                req.log(200);
                Resolved::new(Status(200), req_method, maybe_path)
            },
            // Handle a TRACE route that exists.
            Method::Trace if route_exists => {
                let maybe_path = maybe_route.unwrap();
                let maybe_path = if maybe_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(maybe_path.clone())
                };

                req.log(200);
                Resolved::new(Status(200), req_method, maybe_path)
            },
            // Handle an OPTIONS route that exists.
            Method::Options if route_exists => {
                let maybe_path = maybe_route.unwrap();
                let maybe_path = if maybe_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(maybe_path.clone())
                };

                req.log(200);
                Resolved::new(Status(200), req_method, maybe_path)
            },
            // Handle a CONNECT route that exists.
            Method::Connect if route_exists => {
                let maybe_path = maybe_route.unwrap();
                let maybe_path = if maybe_path.as_os_str().is_empty() {
                    None
                } else {
                    Some(maybe_path.clone())
                };

                req.log(200);
                Resolved::new(Status(200), req_method, maybe_path)
            },
            // Handle routes that do not exist.
            _ => {
                req.log(404);
                Resolved::new(Status(404), req_method, self.get_error_page())
            },
        }
    }
}

// TODO: Just construct a `Response` directly instead of using a `Resolved` object.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub status: Status,
    pub method: Method,
    pub path: Option<PathBuf>,
}

impl Resolved {
    #[must_use]
    pub const fn new(status: Status, method: Method, path: Option<PathBuf>) -> Self {
        Self { status, method, path }
    }

    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    #[must_use]
    pub const fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
}
