use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{Method, Request, Status};

#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct Route {
    pub method: Method,
    pub uri: String,
}

impl Route {
    #[must_use]
    pub fn new(method: Method, uri: &str) -> Self {
        let uri = uri.to_string();
        Self { method, uri }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Router {
    pub routes: BTreeMap<Route, PathBuf>,
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
        self.routes.get(&req.route()).map_or_else(
            || {
                // No local resource path found for the URI.
                req.log_with_status(404);
                Resolved::new(Status(404), self.get_error_page())
            },
            // Resolved the route to a local resource path.
            |path| {
                req.log_with_status(200);
                Resolved::new(Status(200), Some(path.clone()))
            },
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub status: Status,
    pub path: Option<PathBuf>,
}

impl Resolved {
    #[must_use]
    pub const fn new(status: Status, path: Option<PathBuf>) -> Self {
        Self { status, path }
    }

    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    #[must_use]
    pub const fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
}
