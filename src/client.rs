use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::{TcpStream, ToSocketAddrs};

use crate::{
    Body, Connection, HeaderName, HeaderValue, Headers, Method, NetError,
    NetResult, Request, RequestLine, Response, Version,
};
use crate::header_name::DATE;
use crate::util;

pub mod output;
pub use output::{WriteCliError, Output};

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
    pub fn method(mut self, method: &Method) -> Self {
        self.method = method.clone();
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
        if !path.is_empty() {
            self.path = Some(path.to_string());
        }

        self
    }

    /// Sets a request header field line.
    #[must_use]
    pub fn header(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Sets the request headers.
    #[must_use]
    pub fn headers(mut self, headers: &Headers) -> Self {
        self.headers = headers.clone();
        self
    }

    /// Sets the request body.
    #[must_use]
    pub fn body(mut self, body: &Body) -> Self {
        if !body.is_empty() {
            self.body = body.clone();
        }

        self
    }

    /// Builds and returns a new `Client` instance.
    #[allow(clippy::missing_errors_doc)]
    pub fn build(mut self) -> NetResult<Client> {
        let Some(addr) = self.addr.as_ref() else {
            return Err(NetError::NotConnected);
        };

        let conn = TcpStream::connect(addr)
            .map_err(Into::into)
            .and_then(Connection::try_from)?;

        self.headers.default_request_headers(&self.body, &conn.remote_addr);

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

    /// Sends a GET request to the provided address.
    ///
    /// # Errors
    ///
    /// Returns an error if building the `Client` or sending the request
    /// fails.
    pub fn get(uri: &str) -> NetResult<Self> {
        util::parse_uri(uri)
            .and_then(|(ref addr, ref path)| {
                ClientBuilder::new()
                    .method(&Method::Get)
                    .addr(addr)
                    .path(path)
                    .send()
            })
    }

    /// Sends a custom SHUTDOWN request to the provided test server address.
    ///
    /// # Errors
    ///
    /// Returns an error if building the `Client` or sending the request
    /// fails.
    pub fn shutdown(addr: &str) -> NetResult<()> {
        ClientBuilder::new()
            .method(&Method::Custom("SHUTDOWN".to_string()))
            .addr(addr)
            .send()?;

        Ok(())
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
            .and_then(|req| self.conn.send_request(req))
    }

    /// Receives an HTTP response from the remote host.
    #[allow(clippy::missing_errors_doc)]
    pub fn recv(&mut self) {
        self.res = self.conn.recv_response().ok();
    }
}
