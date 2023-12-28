use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};

use crate::{
    Body, Connection, Headers, Method, NetError, NetResult, Path, Request,
    RequestLine, Response, Version,
};
use crate::header_name::DATE;
use crate::util;

pub mod cli;
pub mod output;

pub use cli::ClientCli;
pub use output::OutputStyle;

/// An HTTP client builder object.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ClientBuilder<A>
where
    A: ToSocketAddrs,
{
    pub debug: bool,
    pub do_send: bool,
    pub method: Option<Method>,
    pub addr: Option<A>,
    pub path: Option<Path>,
    pub version: Option<Version>,
    pub headers: Option<Headers>,
    pub body: Option<Body>,
    pub output: Option<OutputStyle>,
}

impl<A> Default for ClientBuilder<A>
where
    A: ToSocketAddrs,
{
    fn default() -> Self {
        Self {
            debug: false,
            do_send: true,
            method: None,
            addr: None,
            path: None,
            version: None,
            headers: None,
            body: None,
            output: None
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

    /// Enable debug printing.
    pub fn debug(&mut self, do_debug: bool) -> &mut Self {
        self.debug = do_debug;
        self
    }

    /// Set whether to send the request..
    pub fn do_send(&mut self, do_send: bool) -> &mut Self {
        self.do_send = do_send;
        self
    }

    /// Sets the HTTP method.
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = Some(method);
        self
    }

    /// Sets the HTTP version.
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    /// Sets the socket address of the remote server.
    pub fn addr(&mut self, addr: A) -> &mut Self {
        self.addr = Some(addr);
        self
    }

    /// Sets the URI path to the target resource.
    pub fn path(&mut self, path: &str) -> &mut Self {
        if !path.is_empty() {
            self.path = Some(path.into());
        }

        self
    }

    /// Inserts a request header.
    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        if self.headers.is_none() {
            self.headers = Some(Headers::new());
        }

        if let Some(headers) = self.headers.as_mut() {
            headers.header(name, value);
        }

        self
    }

    /// Sets the request headers.
    pub fn headers(&mut self, headers: Headers) -> &mut Self {
        self.headers = Some(headers);
        self
    }

    /// Sets the request body.
    pub fn body(&mut self, body: Body) -> &mut Self {
        if body.is_empty() {
            self.body = Some(Body::Empty);
        } else {
            self.body = Some(body);
        }

        self
    }

    /// Sets the request body.
    pub fn output(&mut self, output: OutputStyle) -> &mut Self {
        self.output = Some(output);
        self
    }

    /// Builds and returns a new `Client`.
    ///
    /// # Errors
    /// 
    /// Returns an error if establishing a TCP connection fails.
    pub fn build(&mut self) -> NetResult<Client> {
        let Some(addr) = self.addr.as_ref() else {
            return Err(NetError::ConnectFailure);
        };

        let conn = TcpStream::connect(addr)
            .map_err(|_| NetError::ConnectFailure)
            .and_then(Connection::try_from)?;

        let method = self.method.take().unwrap_or_default();
        let path = self.path.take().unwrap_or_default();
        let version = self.version.take().unwrap_or_default();
        let headers = self.headers.take().unwrap_or_default();
        let body = self.body.take().unwrap_or_default();

        let req = Request {
            request_line: RequestLine {
                method,
                path,
                version
            },
            headers,
            body
        };

        let client = Client {
            debug: false,
            do_send: true,
            req: Some(req),
            res: None,
            conn: Some(conn),
            output: self.output.unwrap_or_default()
        };

        Ok(client)
    }

    /// Sends an HTTP `Request` and returns the `Client`.
    ///
    /// # Errors
    /// 
    /// Returns an error if building the `Client` or sending the `Request`
    /// fails.
    pub fn send(&mut self) -> NetResult<Client> {
        let mut client = self.build()?;
        client.send_request()?;
        Ok(client)
    }
}

/// An HTTP client.
#[derive(Debug)]
pub struct Client {
    pub debug: bool,
    pub do_send: bool,
    pub req: Option<Request>,
    pub res: Option<Response>,
    pub conn: Option<Connection>,
    pub output: OutputStyle,
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

impl Default for Client {
    fn default() -> Self {
        Self {
            debug: false,
            do_send: true,
            req: None,
            res: None,
            conn: None,
            output: OutputStyle::default()
        }
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

    /// Returns a new `ClientBuilder` instance.
    ///
    /// # Errors
    ///
    /// Returns an error `TcpStream::connect` is unable to connect to the the
    /// given `addr`.
    pub fn new(method: Method, addr: &str, path: &str) -> NetResult<Self> {
        ClientBuilder::new()
            .method(method)
            .addr(addr)
            .path(path)
            .build()
    }

    /// Returns a new `ClientBuilder` instance.
    /// 
    /// # Errors
    /// 
    /// Returns an error `TcpStream::connect` is unable to connect to the the
    /// given `addr`.
    pub fn send(method: Method, addr: &str, path: &str) -> NetResult<Self> {
        ClientBuilder::new()
            .method(method)
            .addr(addr)
            .path(path)
            .send()
    }

    /// Sends a GET request to the provided address.
    ///
    /// # Errors
    ///
    /// Returns an error `TcpStream::connect` is unable to connect to the the
    /// given `addr` or if sending the request fails.
    pub fn get(uri: &str) -> NetResult<()> {
        let (addr, path) = util::parse_uri(uri)?;

        let mut client = ClientBuilder::new()
            .method(Method::Get)
            .addr(&addr)
            .path(&path)
            .build()?;

        client.output.format_str("b");

        client.send_request()
    }

    /// Sends a custom SHUTDOWN request to the provided test server address.
    ///
    /// # Errors
    ///
    /// Returns an error `TcpStream::connect` is unable to connect to the the
    /// given `addr` or if sending the request fails.
    pub fn shutdown(addr: &str) -> NetResult<()> {
        let mut client = ClientBuilder::new()
            .method(Method::Custom("SHUTDOWN".to_string()))
            .addr(addr)
            .build()?;

        client.output.format_str("b");

        client.send_request()
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

    /// Prints the request and the response to the provided `BufWriter`,
    /// based on the output style settings.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the request to stdout fails.
    pub fn print<W: Write>(&mut self, out: &mut BufWriter<W>) -> NetResult<()> {
        // Ignore Date headers.
        if self.output.no_dates {
            self.remove_date_headers();
        }

        // Handle request output.
        if let Some(req) = self.req.as_ref() {
            self.output.print_request_line(&req.request_line, out)?;
            self.output
                .print_headers(&req.headers, &self.output.req_style, out)?;
            self.output.print_body(&req.body, &self.output.req_style, out)?;
        }

        // Handle response output.
        if let Some(res) = self.res.as_ref() {
            if self.output.include_separator() {
                writeln!(out)?;
            }

            self.output.print_status_line(&res.status_line, out)?;
            self.output
                .print_headers(&res.headers, &self.output.res_style, out)?;

            if self.req
                .as_ref()
                .is_some_and(|req| !req.route().is_head())
            {
                self.output.print_body(&res.body, &self.output.res_style, out)?;
            }
        }

        writeln!(out)?;
        out.flush()?;
        Ok(())
    }

    /// Writes an HTTP `Request` to a `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::send_request` fails.
    pub fn send_request(&mut self) -> NetResult<()> {
        let req = self.req.as_mut().ok_or(NetError::NoRequest)?;

        self.conn
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(|conn| conn.send_request(req))
    }

    /// Writes an HTTP `Response` to a `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::send_response` fails.
    pub fn send_response(&mut self) -> NetResult<()> {
        let res = self.res.as_mut().ok_or(NetError::NoResponse)?;

        self.conn
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(|conn| conn.send_response(res))
    }

    /// Reads and parse an HTTP `Request` from the contained `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::recv_request` fails.
    pub fn recv_request(&mut self) -> NetResult<()> {
        let req = self.conn
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(Connection::recv_request)?;

        self.req = Some(req);

        Ok(())
    }

    /// Reads and parses an HTTP `Response` from the contained `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::recv_response` fails.
    pub fn recv_response(&mut self) -> NetResult<()> {
        let res = self
            .conn
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(Connection::recv_response)?;

        self.res = Some(res);

        Ok(())
    }
}
