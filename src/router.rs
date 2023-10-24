use std::{
    collections::HashMap,
    path::PathBuf,
};

use crate::{Method, Request, Status};

type RoutesMap = HashMap<String, (Vec<Method>, PathBuf)>;

#[derive(Debug)]
pub struct Router {
    routes: RoutesMap,
}

impl Default for Router {
    fn default() -> Self {
        let routes: RoutesMap = HashMap::new();
        Self { routes }
    }
}

impl Router {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mount<P: Into<PathBuf>>(
        &mut self,
        method: Method,
        uri: &str,
        path: P
    ) {
        self.routes
            .entry(String::from(uri))
            .and_modify(|data| {
                if !data.0.contains(&method) {
                    data.0.push(method.clone());
                }
            })
            .or_insert_with(|| {
                let uri_methods: Vec<Method> = vec![method.clone()];
                let path: PathBuf = path.into();
                (uri_methods, path)
            });
    }

    pub fn set_favicon<P: Into<PathBuf>>(&mut self, path: P) {
        let path: PathBuf = path.into();
        let key = String::from("/favicon.ico");
        let uri_methods: Vec<Method> = vec![Method::Get];
        self.routes.insert(key, (uri_methods, path));
    }

    fn get_error_page(&self) -> Option<PathBuf> {
        self.routes.get("__error").map(|data| data.1.clone())
    }

    pub fn set_error_page<P: Into<PathBuf>>(&mut self, path: P) {
        let path: PathBuf = path.into();
        let key = String::from("__error");
        let uri_methods: Vec<Method> = vec![];
        self.routes.insert(key, (uri_methods, path));
    }

    #[must_use]
    pub fn resolve(&self, req: &Request) -> (Status, Option<PathBuf>) {
        match self.routes.get(&*req.uri()) {
            // Resolved the route successfully.
            Some(data) if data.0.contains(req.method()) => {
                (Status(200), Some(data.1.clone()))
            },
            // URI was found but not for that HTTP method.
            Some(_) => (Status(400), None),
            // No resource found for the URI.
            None => (Status(404), self.get_error_page()),
        }
    }
}
