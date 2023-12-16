use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufWriter, Result as IoResult, Write, WriterPanicked};
use std::net::TcpStream;
use std::str::FromStr;
use std::string::ToString;

use crate::consts::{ACCEPT, CONNECTION, HOST, SERVER, USER_AGENT, WRITER_BUFSIZE};
use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetError, NetReader, NetResult, ParseErrorKind,
    Request, Route, Router, Status, Target, Version,
};

/// A buffered writer wrapper around a `TcpStream` instance.
#[derive(Debug)]
pub struct NetWriter(pub BufWriter<TcpStream>);

impl From<TcpStream> for NetWriter {
    fn from(stream: TcpStream) -> Self {
        Self(BufWriter::with_capacity(WRITER_BUFSIZE, stream))
    }
}

impl From<NetReader> for NetWriter {
    fn from(reader: NetReader) -> Self {
        Self::from(reader.into_inner())
    }
}

impl Write for NetWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.0.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.0.write_all(buf)
    }
}

impl NetWriter {
    /// Returns a clone of the current `NetWriter` instance.
    #[allow(clippy::missing_errors_doc)]
    pub fn try_clone(&self) -> NetResult<Self> {
        let stream = self.get_ref().try_clone()?;
        Ok(Self::from(stream))
    }

    /// Consumes the `NetWriter` and returns the underlying `TcpStream`.
    pub fn into_parts(self) -> (TcpStream, Result<Vec<u8>, WriterPanicked>) {
        self.0.into_parts()
    }

    /// Consumes the `NetWriter` and returns the underlying `TcpStream`.
    #[allow(clippy::missing_errors_doc)]
    pub fn into_inner(self) -> NetResult<TcpStream> {
        self.0.into_inner().map_err(|e| e.into_error().into())
    }

    /// Returns a reference to the underlying `TcpStream`.
    #[must_use]
    pub fn get_ref(&self) -> &TcpStream {
        self.0.get_ref()
    }

    /// Writes an HTTP request to the underlying `TcpStream`.
    #[allow(clippy::missing_errors_doc)]
    pub fn send_request(&mut self, req: &mut Request) -> NetResult<()> {
        if !req.headers.contains(&ACCEPT) {
            req.headers.insert_accept("*/*");
        }

        if !req.headers.contains(&HOST) {
            let stream = self.get_ref();
            let remote = stream.peer_addr()?;
            req.headers.insert_host(remote.ip(), remote.port());
        }

        if !req.headers.contains(&USER_AGENT) {
            req.headers.insert_user_agent();
        }

        self.write_all(format!("{}\r\n", &req.request_line).as_bytes())?;
        self.write_headers(&req.headers)?;
        self.write_body(&req.body)?;
        self.flush()?;
        Ok(())
    }

    /// Writes a server error HTTP response to the underlying `TcpStream` with
    /// an error message included in the body.
    #[allow(clippy::missing_errors_doc)]
    pub fn send_server_error(&mut self, msg: &str) -> NetResult<()> {
        let mut res = Response::new(500);
        res.body = Body::Text(msg.to_owned());
        res.headers.insert_connection("close");
        res.headers.insert_cache_control("no-cache");
        res.headers.insert_content_length(res.body.len());
        res.headers.insert_content_type("text/plain; charset=utf-8");
        self.send_response(&mut res)
    }

    /// Writes a server error response to the underlying `TcpStream`.
    #[allow(clippy::missing_errors_doc)]
    pub fn send_status(&mut self, code: u16) -> NetResult<()> {
        let mut res = Response::new(code);
        res.headers.insert_cache_control("no-cache");
        res.headers.insert_connection("close");
        self.send_response(&mut res)?;
        Ok(())
    }

    /// Writes an HTTP response to the underlying `TcpStream`.
    #[allow(clippy::missing_errors_doc)]
    pub fn send_response(&mut self, res: &mut Response) -> NetResult<()> {
        if !res.headers.contains(&SERVER) {
            res.headers.insert_server();
        }

        self.write_all(format!("{}\r\n", &res.status_line).as_bytes())?;
        self.write_headers(&res.headers)?;
        self.write_body(&res.body)?;
        self.flush()?;
        Ok(())
    }

    /// Writes the response headers to the underlying `TcpStream`.
    #[allow(clippy::missing_errors_doc)]
    pub fn write_headers(&mut self, headers: &Headers) -> NetResult<()> {
        if !headers.is_empty() {
            for (name, value) in &headers.0 {
                self.write_all(format!("{name}: {value}\r\n").as_bytes())?;
            }
        }

        self.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes the response body to the underlying `TcpStream`.
    #[allow(clippy::missing_errors_doc)]
    pub fn write_body(&mut self, body: &Body) -> NetResult<()> {
        if !body.is_empty() {
            self.write_all(body.as_bytes())?;
        }

        Ok(())
    }
}

/// Represents the status line of an HTTP response.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusLine {
    pub version: Version,
    pub status: Status,
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            version: Version::OneDotOne,
            status: Status(200),
        }
    }
}

impl Display for StatusLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {}", self.version, self.status)
    }
}

impl FromStr for StatusLine {
    type Err = NetError;

    fn from_str(line: &str) -> NetResult<Self> {
        line.find("HTTP")
            .ok_or(NetError::ParseError(ParseErrorKind::StatusLine))
            .and_then(|start| {
                line[start..]
                    .split_once(' ')
                    .ok_or(NetError::ParseError(ParseErrorKind::StatusLine))
                    .and_then(|(token1, token2)| {
                        let version = token1.parse::<Version>()?;
                        let status = token2.parse::<Status>()?;
                        Ok(Self::new(version, status))
                    })
            })
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
}

/// Represents the components of an HTTP response.
pub struct Response {
    pub status_line: StatusLine,
    pub headers: Headers,
    pub body: Body,
}

impl PartialEq for Response {
    fn eq(&self, other: &Self) -> bool {
        self.status_line == other.status_line
            && self.headers == other.headers
            && self.body == other.body
    }
}

impl Eq for Response {}

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
            .finish()
    }
}

impl Response {
    /// Resolves a `Route` into a `Response` based on the provided `Router`.
    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::match_same_arms)]
    #[must_use]
    pub fn from_route(route: &Route, router: &Router) -> Self {
        if router.is_empty() {
            let mut res = Self::new(500);
            res.body = Body::Text("This server has no routes configured.".to_string());
            res.headers.insert_cache_control("no-cache");
            res.headers.insert_connection("close");
            res.headers.insert_content_length(res.body.len());
            res.headers.insert_content_type("text/plain; charset=utf-8");
            return res;
        }

        let method = route.method();
        let maybe_target = router.resolve(route);

        let maybe_res = match (maybe_target, method) {
            (Some(target), Method::Get) => Self::from_target(200, target),
            (Some(target), Method::Head) => Self::from_target(200, target),
            (Some(target), Method::Post) => Self::from_target(201, target),
            (Some(target), Method::Put) => Self::from_target(200, target),
            (Some(target), Method::Patch) => Self::from_target(200, target),
            (Some(target), Method::Delete) => Self::from_target(200, target),
            (Some(target), Method::Trace) => Self::from_target(200, target),
            (Some(target), Method::Options) => Self::from_target(200, target),
            (Some(target), Method::Connect) => Self::from_target(200, target),
            (None, Method::Head) => {
                // Allow HEAD requests for any route configured for a GET request.
                let get_route = Route::Get(route.path().to_string());

                router.resolve(&get_route).map_or_else(
                    // No route exists for a GET request either.
                    || {
                        router.get_error_404().map_or_else(
                            || Self::from_target(404, &Target::Empty),
                            |target| Self::from_target(404, target),
                        )
                    },
                    // GET route exists so send it as a HEAD response.
                    |target| Self::from_target(200, target),
                )
            }
            // Handle routes that do not exist.
            (None, _) => router.get_error_404().map_or_else(
                || Self::from_target(404, &Target::Empty),
                |target| Self::from_target(404, target),
            ),
        };

        match maybe_res {
            Ok(mut res) if method == Method::Head => {
                res.body = Body::Empty;
                res
            }
            Ok(res) => res,
            Err(e) => {
                let mut res = Self::new(500);
                res.body = Body::Text(format!("Error: {e}"));
                res.headers.insert_cache_control("no-cache");
                res.headers.insert_connection("close");
                res.headers.insert_content_length(res.body.len());
                res.headers.insert_content_type("text/plain; charset=utf-8");
                res
            }
        }
    }

    /// Parses the target type and returns a new `Response` object.
    #[must_use]
    pub fn new(code: u16) -> Self {
        Self {
            status_line: StatusLine {
                status: Status(code),
                version: Version::OneDotOne,
            },
            headers: Headers::new(),
            body: Body::Empty,
        }
    }

    /// Parses the target type and returns a new `Response` object.
    ///
    /// # Errors
    ///
    /// Returns an error if `fs::read` or `fs::read_to_string` fails.
    pub fn from_target(code: u16, target: &Target) -> NetResult<Self> {
        let mut res = Self::new(code);

        if let Some(header) = target.as_content_type_header() {
            res.headers.insert(header.name, header.value);
        }

        res.headers.insert_cache_control("no-cache");

        res.body = match target {
            Target::Empty => Body::Empty,
            Target::Text(s) => Body::Text((*s).to_string()),
            Target::Html(s) => Body::Html((*s).to_string()),
            Target::Json(s) => Body::Json((*s).to_string()),
            Target::Xml(s) => Body::Xml((*s).to_string()),
            Target::File(ref fpath) => Body::try_from(fpath)?,
            Target::Favicon(ref fpath) => {
                res.headers.insert_cache_control("max-age=604800");
                Body::try_from(fpath)?
            }
            Target::Bytes(ref bytes) => Body::Bytes(bytes.clone()),
        };

        if !res.body.is_empty() {
            res.headers.insert_content_length(res.body.len());
        }

        Ok(res)
    }

    /// Returns a String representation of the response's status line.
    #[must_use]
    pub fn status_line(&self) -> String {
        self.status_line.to_string()
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

    /// Returns the response headers.
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
            self.headers
                .0
                .iter()
                .fold(String::new(), |mut acc, (name, value)| {
                    acc.push_str(&format!("{name}: {value}\n"));
                    acc
                })
        }
    }

    /// Returns true if the Connection header is present with the value "close".
    #[must_use]
    pub fn has_closed_connection_header(&self) -> bool {
        matches!(
            self.headers.get(&CONNECTION),
            Some(conn_val) if conn_val.as_str().eq_ignore_ascii_case("close")
        )
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

    /// Sends an HTTP response to a remote client.
    #[allow(clippy::missing_errors_doc)]
    pub fn send(&mut self, writer: &mut NetWriter) -> NetResult<()> {
        writer.send_response(self)
    }

    /// Receives an HTTP response from a remote server.
    #[allow(clippy::missing_errors_doc)]
    pub fn recv(reader: &mut NetReader) -> NetResult<Self> {
        reader.recv_response()
    }
}
