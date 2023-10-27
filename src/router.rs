use std::{
    collections::HashMap,
    path::PathBuf,
};

use crate::{Method, Request, Status};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Route {
    pub method: Method,
    pub uri: String,
}

impl Route {
    pub fn new(method: Method, uri: &str) -> Self {
        let uri = uri.to_string();
        Self { method, uri }
    }
}

type RoutesMap = HashMap<Route, PathBuf>;

#[derive(Clone, Debug)]
pub struct Router {
    pub routes: RoutesMap,
}

impl Router {
    #[must_use]
    pub fn new() -> Self {
        Self { routes: HashMap::new() }
    }

    pub fn mount(&mut self, route: Route) {
        self.routes.insert(route, PathBuf::new());
    }

    pub fn mount_with_path<P: Into<PathBuf>>(&mut self, route: Route, path: P) {
        let path = path.into();
        self.routes.insert(route, path);
    }

    pub fn get_error_page(&self) -> Option<PathBuf> {
        let route = Route::new(Method::Get, "__error");
        self.routes.get(&route).cloned()
    }

    pub fn resolve(&self, req: &Request) -> Resolved {
        match self.routes.get(&req.route()) {
            // Resolved the route to a local resource path.
            Some(ref path) => {
                req.log_connection_status(200);
                Resolved::new(Status(200), Some(path.to_path_buf()))
            },
            // No local resource path found for the URI.
            None => {
                req.log_connection_status(404);
                Resolved::new(Status(404), self.get_error_page())
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolved {
    pub status: Status,
    pub path: Option<PathBuf>,
}

impl Resolved {
    pub fn new(status: Status, path: Option<PathBuf>) -> Self {
        Self { status, path }
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }
}
