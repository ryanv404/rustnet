use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::{TcpStream, ToSocketAddrs};

use crate::header::{
    ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, DATE, HOST, USER_AGENT,
};
use crate::{
    Body, Connection, HeaderName, HeaderValue, Headers, Method, NetError,
    NetResult, Request, RequestLine, Response, Version,
};

/// An HTTP request builder object.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientBuilder<A>
where
    A: ToSocketAddrs,
{
    pub method: Method,
    pub addr: Option<A>,
    pub path: Option<String>,
    pub version: Version,
    pub headers: Headers,
    pub body: Body,
}

impl<A> Default for ClientBuilder<A>
where
    A: ToSocketAddrs,
{
    fn default() -> Self {
        Self {
            method: Method::Get,
            addr: None,
            path: None,
            version: Version::OneDotOne,
            headers: Headers::new(),
            body: Body::Empty,
        }
    }
}

impl<A> ClientBuilder<A>
where
    A: ToSocketAddrs,
{
    /// Returns a new `ClientBuilder` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP method.
    #[must_use]
    pub const fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Sets the socket address of the remote server.
    #[must_use]
    pub fn addr(mut self, addr: A) -> Self {
        self.addr = Some(addr);
        self
    }

    /// Sets the URI path to the target resource.
    #[must_use]
    pub fn path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    /// Sets the protocol version.
    #[must_use]
    pub const fn version(mut self, version: Version) -> Self {
        self.version = version;
        self
    }

    /// Sets a request header field line.
    #[must_use]
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Sets the request body, Content-Type header, and Content-Length header.
    #[must_use]
    pub fn body(mut self, body: Body, content_type: &str) -> Self {
        if body.is_empty() {
            self.headers.content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = body;
            self.headers.content_length(self.body.len());
            self.headers.content_type(content_type);
        }

        self
    }

    /// Sets a text request body and sets the Content-Type and Content-Length
    /// headers.
    #[must_use]
    pub fn text(mut self, text: &str) -> Self {
        if text.is_empty() {
            self.headers.content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = Body::Text(text.into());
            self.headers.content_length(self.body.len());
            self.headers.content_type("text/plain; charset=utf-8");
        }

        self
    }

    /// Sets a HTML request body and sets the Content-Type and Content-Length
    /// headers.
    #[must_use]
    pub fn html(mut self, html: &str) -> Self {
        if html.is_empty() {
            self.headers.content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = Body::Html(html.into());
            self.headers.content_length(self.body.len());
            self.headers.content_type("text/html; charset=utf-8");
        }

        self
    }

    /// Sets a JSON request body and sets the Content-Type and Content-Length
    /// headers.
    #[must_use]
    pub fn json(mut self, json: &str) -> Self {
        if json.is_empty() {
            self.headers.content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = Body::Json(json.into());
            self.headers.content_length(self.body.len());
            self.headers.content_type("application/json");
        }

        self
    }

    /// Sets a request body comprised of bytes and sets the Content-Type and
    /// Content-Length headers.
    #[must_use]
    pub fn bytes(mut self, bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            self.headers.content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = Body::Bytes(bytes.to_vec());
            self.headers.content_type("application/octet-stream");
        }

        self
    }

    /// Builds and returns a new `Client` instance.
    #[allow(clippy::missing_errors_doc)]
    pub fn build(mut self) -> NetResult<Client> {
        let conn = self.addr.as_ref().ok_or(NetError::NotConnected).and_then(
            |addr| {
                TcpStream::connect(addr)
                    .map_err(|_| NetError::NotConnected)
                    .and_then(Connection::try_from)
            },
        )?;

        if !self.headers.contains(&ACCEPT) {
            self.headers.accept("*/*");
        }

        if !self.headers.contains(&CONTENT_LENGTH) {
            self.headers.content_length(self.body.len());
        }

        if !self.headers.contains(&CONTENT_TYPE) && !self.body.is_empty() {
            self.headers.content_type("text/plain");
        }

        if !self.headers.contains(&HOST) {
            self.headers.host(&conn.remote_addr);
        }

        if !self.headers.contains(&USER_AGENT) {
            self.headers.user_agent("rustnet/0.1");
        }

        let path = self
            .path
            .take()
            .unwrap_or_else(|| String::from("/"));

        let request_line = RequestLine {
            method: self.method,
            path,
            version: self.version,
        };

        let req = Some(Request {
            request_line,
            headers: self.headers,
            body: self.body,
        });

        let res = None;

        Ok(Client { req, res, conn })
    }

    /// Sends an HTTP request and then returns a `Client` instance.
    #[allow(clippy::missing_errors_doc)]
    pub fn send(self) -> NetResult<Client> {
        let mut client = self.build()?;
        client.send()?;
        Ok(client)
    }

    /// Sends an HTTP request with a text body.
    #[allow(clippy::missing_errors_doc)]
    pub fn send_text(self, text: &str) -> NetResult<Client> {
        let mut client = self.text(text).build()?;
        client.send()?;
        Ok(client)
    }

    /// Sends an HTTP request with an HTML body.
    #[allow(clippy::missing_errors_doc)]
    pub fn send_html(self, html: &str) -> NetResult<Client> {
        let mut client = self.html(html).build()?;
        client.send()?;
        Ok(client)
    }

    /// Sends an HTTP request with a JSON body.
    #[allow(clippy::missing_errors_doc)]
    pub fn send_json(self, json: &str) -> NetResult<Client> {
        let mut client = self.json(json).build()?;
        client.send()?;
        Ok(client)
    }
}

/// An HTTP client.
#[derive(Debug)]
pub struct Client {
    pub req: Option<Request>,
    pub res: Option<Response>,
    pub conn: Connection,
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if let Some(req) = self.req.as_ref() {
            req.fmt(f)?;
        }

        if let Some(res) = self.res.as_ref() {
            res.fmt(f)?;
        }

        Ok(())
    }
}

impl Client {
    /// Returns a new `ClientBuilder` instance.
    #[must_use]
    pub fn builder<A>() -> ClientBuilder<A>
    where
        A: ToSocketAddrs,
    {
        ClientBuilder::new()
    }

    /// Removes Date header field entries from requests and responses.
    pub fn remove_date_headers(&mut self) {
        if let Some(req) = self.req.as_mut() {
            req.headers.remove(&DATE);
        }

        if let Some(res) = self.res.as_mut() {
            res.headers.remove(&DATE);
        }
    }

    /// Sends an HTTP request to a remote host.
    #[allow(clippy::missing_errors_doc)]
    pub fn send(&mut self) -> NetResult<()> {
        self.req
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(|req| self.conn.writer.send_request(req))
    }

    /// Receives an HTTP response from the remote host.
    #[allow(clippy::missing_errors_doc)]
    pub fn recv(&mut self) {
        self.res = self.conn.reader.recv_response().ok();
    }
}