use std::fs;
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::consts::{
    CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE,
};
use crate::{
    Headers, HeaderValue, Method, NetResult, Request, Response,
    Status, StatusLine, Version,
};

/// Represents an endpoint defined by an HTTP method and a URI path.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub enum Route {
    Get(String),
    Head(String),
    Post(String),
    Put(String),
    Patch(String),
    Delete(String),
    Trace(String),
    Options(String),
    Connect(String),
}

impl Route {
    /// Constructs a new `Route` instance.
    #[must_use]
    pub fn new(method: Method, uri_path: &str) -> Self {
        let path = uri_path.to_string();

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
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Router(pub BTreeMap<Route, Target>);

impl Router {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mount(&mut self, route: Route, target: Target) {
        self.0.insert(route, target);
    }

    #[must_use]
    pub fn get_target(&self, route: &Route) -> Option<&Target> {
        self.0.get(route)
    }

    /// Returns true if there is an entry associated with `Route`.
    #[must_use]
    pub fn route_exists(&self, route: &Route) -> bool {
        self.0.contains_key(route)
    }

    /// Returns true if the `Router` contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the target resource for error 404 responses.
    #[must_use]
    pub fn get_error_page(&self) -> &Target {
        let route = Route::new(Method::Get, "__error");
        self.get_target(&route).unwrap_or(&Target::Empty)
    }

    /// Resolves a `Request` into a `Response` based on the provided `Router`.
    pub fn resolve(
        req: &Request,
        router: &Arc<Self>,
    ) -> NetResult<Response> {
        if router.is_empty() {
            let res = Self::make_response(
                Status(502),
                Method::Get,
                &Target::Text("This server has no routes configured."),
                req
            )?;

            return Ok(res);
        }

        let (status, method, target) = {
            match (router.get_target(&req.route()), req.method()) {
                (Some(target), Method::Get) => {
                    (Status(200), Method::Get, target)
                },
                (Some(target), Method::Head) => {
                    (Status(200), Method::Head, target)
                },
                (Some(target), Method::Post) => {
                    (Status(200), Method::Post, target)
                },
                (Some(target), Method::Put) => {
                    (Status(200), Method::Put, target)
                },
                (Some(target), Method::Patch) => {
                    (Status(200), Method::Patch, target)
                },
                (Some(target), Method::Delete) => {
                    (Status(200), Method::Delete, target)
                },
                (Some(target), Method::Trace) => {
                    (Status(200), Method::Trace, target)
                },
                (Some(target), Method::Options) => {
                    (Status(200), Method::Options, target)
                },
                (Some(target), Method::Connect) => {
                    (Status(200), Method::Connect, target)
                },
                (None, Method::Head) => {
                    // Handle a HEAD request for a route that does not exist
                    // but does exist as for a GET request.
                    let route = Route::new(Method::Get, req.path());
                    router.get_target(&route).map_or_else(
                        || {
                            // No route exists for a GET request either.
                            (Status(404), Method::Head, router.get_error_page())
                        },
                        |target| {
                            // GET route exists so send it as a HEAD response.
                            (Status(200), Method::Head, target)
                        })
                },
                (None, method) => {
                    // Handle routes that do not exist.
                    (Status(404), method, router.get_error_page())
                },
            }
        };

        Self::make_response(status, method, target, req)
    }

    /// Returns a `Response` object from the resolved route information.
    pub fn make_response(
        status: Status,
        method: Method,
        target: &Target,
        req: &Request,
    ) -> NetResult<Response> {
        let status_line = StatusLine::new(Version::OneDotOne, status);
        let headers = Headers::new();
        let body = None;

        let conn = req.conn.try_clone()?;

        let mut res = Response {
            method,
            status_line,
            headers,
            body,
            conn
        };

        match target {
            Target::File(ref filepath) => {
                let content = fs::read(filepath)?;
                let cont_type = HeaderValue::infer_content_type(filepath);

                res.headers.insert(CONTENT_TYPE, cont_type);
                res.headers.insert(CONTENT_LENGTH, content.len().into());
                res.headers.insert(CACHE_CONTROL, Vec::from("max-age=604800").into());

                res.body = Some(content);
            },
            Target::Handler(handler) => {
                // Call handler to update the response.
                (handler.lock().unwrap())(req, &mut res);

                if res.body.is_some() {
                    res.headers.insert(CACHE_CONTROL, Vec::from("no-cache").into());
                }
            },
            Target::Text(text) => {
                res.headers.insert(CACHE_CONTROL, Vec::from("no-cache").into());
                res.headers.insert(CONTENT_TYPE, Vec::from("text/plain; charset=utf-8").into());
                res.headers.insert(CONTENT_LENGTH, text.len().into());
                res.body = Some(Vec::from(*text));
            },
            Target::Empty => {
                res.headers.insert(CACHE_CONTROL, Vec::from("no-cache").into());
            },
        }

        if res.method == Method::Head {
            res.body = None;
        }

        Ok(res)
    }
}

type RouteHandler = dyn FnMut(&Request, &mut Response) + Send + Sync + 'static;

/// Target resources used by server end-points.
#[derive(Clone)]
pub enum Target {
    Empty,
    File(PathBuf),
    Text(&'static str),
    Handler(Arc<Mutex<RouteHandler>>),
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
            Self::File(ref path) => write!(f, "Target::File({})", path.display()),
            Self::Text(s) => write!(f, "Target::Text({s})"),
            Self::Handler(_) => write!(f, "Target::Handler(...)"),
        }
    }
}

impl PartialEq for Target {
    #[allow(clippy::match_like_matches_macro)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::File(_), Self::File(_)) => true,
            (Self::Text(_), Self::Text(_)) => true,
            (Self::Handler(_), Self::Handler(_)) => true,
            _ => false,
        }
    }
}

impl Eq for Target {}

impl Target {
    /// Returns a default `Target` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the URI target type is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns true if the URI target type is text.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns true if the URI target type is a file.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }

    /// Returns true if the URI target type is handler function.
    #[must_use]
    pub const fn is_handler(&self) -> bool {
        matches!(self, Self::Handler(_))
    }
}

/// A respresentation of the body content type.
#[derive(Clone, Debug)]
pub enum Body {
    Empty,
    Text(String),
    File(PathBuf),
    Bytes(Vec<u8>),
}

impl Default for Body {
    fn default() -> Self {
        Self::Empty
    }
}

impl PartialEq for Body {
    #[allow(clippy::match_like_matches_macro)]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Empty, Self::Empty) => true,
            (Self::Text(_), Self::Text(_)) => true,
            (Self::File(_), Self::File(_)) => true,
            (Self::Bytes(_), Self::Bytes(_)) => true,
            _ => false,
        }
    }
}

impl Eq for Body {}

impl Body {
    /// Returns a default `Body` instance.
    #[must_use]
    pub const fn new() -> Self {
        Self::Empty
    }

    /// Returns true if the body type is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }

    /// Returns true if the body type is bytes.
    #[must_use]
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    /// Returns true if the body type is text.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Returns true if the body type is a file.
    #[must_use]
    pub const fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }
}
