use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::ErrorKind as IoErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::string::ToString;

use crate::consts::{CONNECTION, CONTENT_TYPE};
use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetReader, NetResult, 
    NetWriter, Request, Route, Router, Status, Target, Version,
};

/// Represents the status line of an HTTP response.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusLine {
    pub version: Version,
    pub status: Status,
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            version: Version::OneDotOne,
            status: Status(200)
        }
    }
}

impl Display for StatusLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {}", self.version, self.status)
    }
}

impl StatusLine {
    /// Returns a new `StatusLine` instance.
    #[must_use]
    pub const fn new(version: Version, status: Status) -> Self {
        Self { version, status }
    }

    /// Returns the protocol version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Returns the response status.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Returns the status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status.code()
    }

    /// Returns the status reason phrase.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status.msg()
    }

    /// Parses a string slice into a `StatusLine` object.
    pub fn parse(line: &str) -> NetResult<Self> {
        let mut tokens = line.trim_start().splitn(3, ' ');
        let version = Version::parse(tokens.next())?;
        let status = Status::parse(tokens.next())?;
        Ok(Self::new(version, status))
    }
}

/// Represents the components of an HTTP response.
pub struct Response {
    pub status_line: StatusLine,
    pub headers: Headers,
    pub body: Body,
    pub writer: Option<NetWriter>,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // The response status line.
        writeln!(f, "{}", self.status_line)?;

        // The response headers.
        for (name, value) in &self.headers.0 {
            writeln!(f, "{name}: {value}")?;
        }

        // The response body.
        if !self.body.is_empty() {
            writeln!(f, "{}", &self.body)?;
        }

        Ok(())
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("Response")
            .field("status_line", &self.status_line)
            .field("headers", &self.headers)
            .field("body", &self.body)
            .field("writer", &self.writer)
			.finish()
	}
}

impl Response {
    /// Resolves a `Request` into a `Response` based on the provided `Router`.
    pub fn from_request(request: &mut Request, router: &Router) -> NetResult<Self> {
        if router.is_empty() {
            let msg = "This server has no routes configured.";
            let target = Target::Text(msg);
            return Self::new(502, &target, request);
        }

        let method = request.method();
        let route = request.route();
        let maybe_target = router.resolve(&route);

        match (maybe_target, method) {
            (Some(target), Method::Get) => {
                Self::new(200, target, request)
            },
            (Some(target), Method::Head) => {
                Self::new(200, target, request)
            },
            (Some(target), Method::Post) => {
                Self::new(201, target, request)
            },
            (Some(target), Method::Put) => {
                Self::new(200, target, request)
            },
            (Some(target), Method::Patch) => {
                Self::new(200, target, request)
            },
            (Some(target), Method::Delete) => {
                Self::new(200, target, request)
            },
            (Some(target), Method::Trace) => {
                Self::new(200, target, request)
            },
            (Some(target), Method::Options) => {
                Self::new(200, target, request)
            },
            (Some(target), Method::Connect) => {
                Self::new(200, target, request)
            },
            (None, Method::Head) => {
                // Allow HEAD requests for any route configured for a GET request.
                let route = Route::Get(request.request_line.path.clone());

                match router.resolve(&route) {
                    // GET route exists so send it as a HEAD response.
                    Some(target) => Self::new(200, target, request),
                    // No route exists for a GET request either.
                    None => Self::new(404, router.error_handler(), request),
                }
            },
            // Handle routes that do not exist.
            (None, _) => Self::new(404, router.error_handler(), request),
        }
    }

    /// Returns a new `Response` object.
    pub fn new(
        code: u16,
        target: &Target,
        req: &mut Request
    ) -> NetResult<Self> {
        let writer = req.reader
            .take()
            .and_then(|reader| NetWriter::try_from(&reader).ok());

        let mut res = Self {
            status_line: StatusLine::new(Version::OneDotOne, Status(code)),
            headers: Headers::new(),
            body: Body::Empty,
            writer
        };

        match target {
            Target::Empty => res.headers.insert_cache_control("no-cache"),
            Target::Text(s) => {
                res.headers.insert_cache_control("no-cache");
                res.headers.insert_content_length(s.len());
                res.headers.insert_content_type("text/plain; charset=utf-8");
                res.body = Body::Text(s.to_string());
            },
            Target::Html(s) => {
                res.headers.insert_cache_control("no-cache");
                res.headers.insert_content_length(s.len());
                res.headers.insert_content_type("text/html; charset=utf-8");
                res.body = Body::Html(s.to_string());
            },
            Target::Json(s) => {
                res.headers.insert_cache_control("no-cache");
                res.headers.insert_content_length(s.len());
                res.headers.insert_content_type("application/json");
                res.body = Body::Json(s.to_string());
            },
            Target::Xml(s) => {
                res.headers.insert_cache_control("no-cache");
                res.headers.insert_content_length(s.len());
                res.headers.insert_content_type("application/xml");
                res.body = Body::Xml(s.to_string());
            },
            Target::File(ref fpath) => {
                let content = fs::read(fpath)?;
                let cont_type = HeaderValue::infer_content_type(fpath);

                res.headers.insert_cache_control("max-age=604800");
                res.headers.insert(CONTENT_TYPE, cont_type);
                res.headers.insert_content_length(content.len());
                res.body = Body::Bytes(content);
            },
            Target::Favicon(ref fpath) => {
                let content = fs::read(fpath)?;

                res.headers.insert_cache_control("max-age=604800");
                res.headers.insert_content_type("image/x-icon");
                res.headers.insert_content_length(content.len());
                res.body = Body::Favicon(content);
            },
            Target::FnMut(handler) => {
                // Call the handler to update the response.
                (handler.lock().unwrap())(req, &mut res);

                if !res.body.is_empty() {
                    res.headers.insert_cache_control("no-cache");
                    res.headers.insert_content_length(res.body.len());
                    res.headers.insert_content_type("text/plain; charset=utf-8");
                }
            },
            // Call the handler to perform an action (with context).
            Target::Fn(handler) => {
                (handler)(req, &res);

                if !res.body.is_empty() {
                    res.headers.insert_cache_control("no-cache");
                    res.headers.insert_content_length(res.body.len());
                    res.headers.insert_content_type("text/plain; charset=utf-8");
                }
            },
            Target::Bytes(ref bytes) => {
                res.headers.insert_cache_control("no-cache");
                res.headers.insert_content_length(bytes.len());
                res.headers.insert_content_type("application/octet-stream");
                res.body = Body::Bytes(bytes.to_vec());
            },
        }

        // Return accurate headers but no body for HEAD requests.
        if req.request_line.method == Method::Head {
            res.body = Body::Empty;
        }

        Ok(res)
    }

    /// Returns the protocol version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.status_line.version
    }

    /// Returns the response's `Status` value.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status_line.status
    }

    /// Returns the status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_line.status.code()
    }

    /// Returns the status reason phrase.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status_line.status.msg()
    }

    /// Returns the `SocketAddr` of the remote half of the connection.
    #[must_use]
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.writer
            .as_ref()
            .and_then(|writer| writer.0.get_ref().peer_addr().ok())
    }

    /// Returns the `IpAddr` of the remote half of the connection.
    #[must_use]
    pub fn remote_ip(&self) -> Option<IpAddr> {
        self.remote_addr().map(|sock| sock.ip())
    }

    /// Returns the port in use by the remote half of the connection.
    #[must_use]
    pub fn remote_port(&self) -> Option<u16> {
        self.remote_addr().map(|sock| sock.port())
    }

    /// Returns the `SocketAddr` of the local half of the connection.
    #[must_use]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.writer
            .as_ref()
            .and_then(|writer| writer.0.get_ref().local_addr().ok())
    }

    /// Returns the `IpAddr` of the local half of the  connection.
    #[must_use]
    pub fn local_ip(&self) -> Option<IpAddr> {
        self.local_addr().map(|sock| sock.ip())
    }

    /// Returns the port in use by the local half of the connection.
    #[must_use]
    pub fn local_port(&self) -> Option<u16> {
        self.local_addr().map(|sock| sock.port())
    }

    /// Returns a map of the response's headers.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Adds or modifies the header field represented by `HeaderName`.
    pub fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    /// Returns the `Header` entry for the given `HeaderName`, if present.
    #[must_use]
    pub fn get_header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Returns the response headers as a String.
    #[must_use]
    pub fn headers_to_string(&self) -> String {
        if self.headers.is_empty() {
            String::new()
        } else {
            self.headers.0.iter().fold(String::new(), 
                |mut acc, (name, value)| {
                    acc.push_str(&format!("{name}: {value}\n"));
                    acc
                })
        }
    }

    /// Returns true if the Connection header is present with the value "close".
    #[must_use]
    pub fn has_closed_connection_header(&self) -> bool {
        self.headers.contains(&CONNECTION)
    }

    /// Returns true if a response body is allowed.
    #[must_use]
    pub fn body_is_permitted(&self, method: Method) -> bool {
        match self.status_code() {
            // 1xx (Informational), 204 (No Content), and 304 (Not Modified).
            100..=199 | 204 | 304 => false,
            // CONNECT responses with a 2xx (Success) status.
            200..=299 if method == Method::Connect => false,
            // HEAD responses.
            _ if method == Method::Head => false,
            _ => true,
        }
    }

    /// Returns a reference to the message body.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }

    /// Returns a String representation of the response's status line.
    #[must_use]
    pub fn status_line(&self) -> String {
        self.status_line.to_string()
    }

    /// Sends an HTTP response to a remote client.
    pub fn send(&mut self) -> NetResult<()> {
        let mut writer = self.writer
            .as_ref()
            .and_then(|writer| writer.try_clone().ok())
            .ok_or_else(|| IoErrorKind::NotConnected)?;

        writer.send_response(self)
    }

    /// Receives an HTTP response from a remote server.
    pub fn recv(mut reader: NetReader) -> NetResult<Response> {
        reader.recv_response()
    }
}
