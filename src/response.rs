use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufWriter, Result as IoResult, Write, WriterPanicked};
use std::net::TcpStream;
use std::str::FromStr;

use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetError, NetReader,
    NetResult, NetParseError, Request, RequestLine, Route, Router, Status,
    Version, WRITER_BUFSIZE,
};

/// A buffered writer responsible for writing to an inner `TcpStream`.
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
    /// Returns a clone of the current `NetWriter`.
    ///
    /// # Errors
    ///
    /// An error is returned if the underlying call to `TcpStream::try_clone`
    /// returns an error.
    pub fn try_clone(&self) -> NetResult<Self> {
        let stream = self.get_ref().try_clone()?;
        Ok(Self::from(stream))
    }

    /// Consumes the `NetWriter` and returns the components of underlying
    /// `TcpStream`.
    pub fn into_parts(self) -> (TcpStream, Result<Vec<u8>, WriterPanicked>) {
        self.0.into_parts()
    }

    /// Consumes the `NetWriter` and returns the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// Returns an error if the inner `TcpStream` could not be returned.
    pub fn into_inner(self) -> NetResult<TcpStream> {
        self.0.into_inner().map_err(|e| e.into_error().into())
    }

    /// Returns a reference to the underlying `TcpStream`.
    #[must_use]
    pub fn get_ref(&self) -> &TcpStream {
        self.0.get_ref()
    }

    /// Writes a `RequestLine` to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the `RequestLine` could not be written
    /// to the `TcpStream` successfully.
    pub fn write_request_line(
        &mut self,
        request_line: &RequestLine
    ) -> NetResult<()> {
        self.write_all(format!("{request_line}\r\n").as_bytes())?;
        Ok(())
    }

    /// Writes a `StatusLine` to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the `StatusLine` could not be written
    /// to the `TcpStream` successfully.
    pub fn write_status_line(
        &mut self,
        status_line: &StatusLine
    ) -> NetResult<()> {
        self.write_all(format!("{status_line}\r\n").as_bytes())?;
        Ok(())
    }

    /// Writes all of the header entries in `Headers` to the underlying
    /// `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if a problem was encountered while writing the
    /// `Headers` to the `TcpStream`.
    pub fn write_headers(&mut self, headers: &Headers) -> NetResult<()> {
        if !headers.is_empty() {
            for (name, value) in &headers.0 {
                self.write_all(format!("{name}: {value}\r\n").as_bytes())?;
            }
        }

        self.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes a `Body` to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the `Body` could not be written
    /// to the `TcpStream` successfully.
    pub fn write_body(&mut self, body: &Body) -> NetResult<()> {
        if !body.is_empty() {
            self.write_all(body.as_bytes())?;
        }
        Ok(())
    }

    /// Writes a `Request` to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to write any of the
    /// individual components of the `Request` to the `TcpStream`.
    pub fn send_request(&mut self, req: &mut Request) -> NetResult<()> {
        use crate::header::{ACCEPT, HOST, USER_AGENT};

        if !req.headers.contains(&ACCEPT) {
            req.headers.accept("*/*");
        }

        if !req.headers.contains(&HOST) {
            let stream = self.get_ref();
            let remote = stream.peer_addr()?;
            req.headers.host(remote.ip(), remote.port());
        }

        if !req.headers.contains(&USER_AGENT) {
            req.headers.user_agent();
        }

        self.write_request_line(&req.request_line)?;
        self.write_headers(&req.headers)?;
        self.write_body(&req.body)?;

        self.flush()?;
        Ok(())
    }

    /// Writes an internal server error `Response` to the underlying
    /// `TcpStream` that contains the provided error message.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to write any of the
    /// individual components of the `Response` to the `TcpStream`.
    pub fn send_server_error(&mut self, err_msg: &str) -> NetResult<()> {
        let mut res = Response::new(500);

        // Update the response headers.
        res.headers.server();
        res.headers.connection("close");
        res.headers.cache_control("no-cache");
        res.headers.content_length(err_msg.len());
        res.headers.content_type("text/plain; charset=utf-8");

        // Include the provided error message.
        res.body = Body::Text(err_msg.into());

        self.write_status_line(&res.status_line)?;
        self.write_headers(&res.headers)?;
        self.write_body(&res.body)?;

        self.flush()?;
        Ok(())
    }

    /// Writes a `Response` to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to write any of the
    /// individual components of the `Response` to the `TcpStream`.
    pub fn send_response(&mut self, res: &mut Response) -> NetResult<()> {
        use crate::header::SERVER;

        if !res.headers.contains(&SERVER) {
            res.headers.server();
        }

        self.write_status_line(&res.status_line)?;
        self.write_headers(&res.headers)?;
        self.write_body(&res.body)?;

        self.flush()?;
        Ok(())
    }
}

/// Contains the components of an HTTP status line.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatusLine {
    pub version: Version,
    pub status: Status,
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
            .ok_or(NetError::Parse(NetParseError::StatusLine))
            .and_then(|start| {
                line[start..].split_once(' ')
                .ok_or(NetError::Parse(NetParseError::StatusLine))
            })
            .and_then(|(token1, token2)| {
                let version = token1.parse::<Version>()?;
                let status = token2.parse::<Status>()?;
                Ok(Self { version, status })
            })
    }
}

impl StatusLine {
    /// Returns the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Returns the `Status`.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Returns the `Status` code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status.code()
    }

    /// Returns the `Status` reason phrase.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status.msg()
    }
}

/// Contains the components of an HTTP response.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Response {
    pub status_line: StatusLine,
    pub headers: Headers,
    pub body: Body,
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{}", self.status_line)?;

        for (name, value) in &self.headers.0 {
            writeln!(f, "{name}: {value}")?;
        }

        if !self.body.is_empty() {
            writeln!(f, "{}", &self.body)?;
        }

        Ok(())
    }
}

impl Response {
    /// Returns a new `Response` containing the provided status code.
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

    /// Constructs a new `Response` based on the `Target` of the requested
    /// `Route`.
    ///
    /// # Errors
    ///
    /// Returns an error if a `Response` could not be constructed from
    /// a `Target`.
    pub fn for_route(route: &Route, router: &Router) -> NetResult<Self> {
        let mut target = router.get_target(route);

        // Implement HEAD routes for all GET routes.
        if target.is_not_found() && route.is_head() {
            if let Route::Head(path) = route {
                let path = path.to_string();
                let get_route = Route::Get(path.into());
                let get_target = router.get_target(&get_route);

                if !get_target.is_not_found() {
                    target = get_target;
                }
            }
        }

        let mut res = if target.is_not_found() {
            target = router.get_404_target();
            Self::new(404)
        } else if route.is_post() {
            Self::new(201)
        } else {
            Self::new(200)
        };

        res.body = Body::try_from(target)?;

        // Set the Cache-Control header.
        if res.body.is_favicon() {
            res.headers.cache_control("max-age=604800");
        } else {
            res.headers.cache_control("no-cache");
        }

        // Set the Content-Type header.
        if let Some(cont_type) = res.body.as_content_type() {
            res.headers.content_type(cont_type);
        }

        // Set the Content-Length header.
        if !res.body.is_empty() {
            res.headers.content_length(res.body.len());
        }

        if route.is_head() {
            res.body = Body::Empty;
        }

        Ok(res)
    }

    /// Returns the `StatusLine` for this `Response`.
    #[must_use]
    pub const fn status_line(&self) -> StatusLine {
        self.status_line
    }

    /// Returns the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.status_line.version
    }

    /// Returns the `Status` for this `Response`.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status_line.status
    }

    /// Returns the `Status` code for this `Response`.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status_line.status.code()
    }

    /// Returns the `Status` reason phrase for this `Response`.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status_line.status.msg()
    }

    /// Returns the headers for this `Response`.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the `HeaderName` key is present.
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
        use crate::header::CONNECTION;

        matches!(
            self.headers.get(&CONNECTION),
            Some(conn_val) if conn_val.as_str().eq_ignore_ascii_case("close")
        )
    }

    /// Returns true if a body is permitted for this `Response`.
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

    /// Returns a reference to the `Body`.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }

    /// Writes an HTTP response to a remote client.
    ///
    /// # Errors
    ///
    /// An error is returned if `NetWriter::send_response` encounters an
    /// error.
    pub fn send(&mut self, writer: &mut NetWriter) -> NetResult<()> {
        writer.send_response(self)
    }

    /// Reads and parses an HTTP response from a remote server.
    ///
    /// # Errors
    ///
    /// An error is returned if `NetReader::recv_response` encounters an
    /// error.
    pub fn recv(reader: &mut NetReader) -> NetResult<Self> {
        reader.recv_response()
    }
}
