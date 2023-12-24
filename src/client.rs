use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs};

use crate::{
    Body, Connection, HeaderName, HeaderValue, Headers, Method, NetError,
    NetResult, Request, RequestLine, Response, Version,
};
use crate::header_name::DATE;
use crate::util;

pub mod output;
pub mod tui;

pub use output::{OutputStyle, WriteCliError};
pub use tui::Tui;

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
    pub output: OutputStyle,
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
            output: OutputStyle::default()
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

    /// Sets the request body.
    #[must_use]
    pub fn output(mut self, output: &OutputStyle) -> Self {
        self.output = output.clone();
        self
    }

    /// Builds and returns a new `Client`.
    ///
    /// # Errors
    /// 
    /// Returns an error if establishing a TCP connection fails.
    pub fn build(mut self) -> NetResult<Client> {
        let Some(addr) = self.addr.as_ref() else {
            return Err(NetError::ConnectFailure);
        };

        let conn = TcpStream::connect(addr)
            .map_err(|_| NetError::ConnectFailure)
            .and_then(Connection::try_from)?;

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

        Ok(Client {
            req,
            res: None,
            conn: Some(conn),
            output: self.output
        })
    }

    /// Sends an HTTP `Request` and returns the `Client`.
    ///
    /// # Errors
    /// 
    /// Returns an error if building the `Client` or sending the `Request`
    /// fails.
    pub fn send(self) -> NetResult<Client> {
        let mut client = self.build()?;
        client.send_request()?;
        Ok(client)
    }
}

/// An HTTP client.
#[derive(Debug, Default)]
pub struct Client {
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
    pub fn get(uri: &str) -> NetResult<()> {
        let mut client = util::parse_uri(uri)
            .and_then(|(ref addr, ref path)| {
                ClientBuilder::new()
                    .method(&Method::Get)
                    .addr(addr)
                    .path(path)
                    .build()
            })?;

        client.output.format_str("b");
        client.send_request()
    }

    /// Sends a custom SHUTDOWN request to the provided test server address.
    ///
    /// # Errors
    ///
    /// Returns an error if building the `Client` or sending the request
    /// fails.
    pub fn shutdown(addr: &str) -> NetResult<()> {
        let mut client = ClientBuilder::new()
            .method(&Method::Custom("SHUTDOWN".to_string()))
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

    /// Prints the request to the provided `BufWriter`, based on the output
    /// style settings.
    ///
    /// # Errors
    ///
    /// Returns an error if writing the request to stdout fails.
    pub fn print_request<W: Write>(
        &self,
        out: &mut BufWriter<W>
    ) -> NetResult<()> {
        if let Some(req) = self.req.as_ref() {
            self.output.print_request_line(req, out)?;
            self.output.print_req_headers(req, out)?;
            self.output.print_req_body(req, out)?;
        }

        Ok(())
    }

    /// Prints the response to the provided `BufWriter`, based on the output
    /// style settings.
    /// 
    /// # Errors
    ///
    /// Returns an error if writing the response to stdout fails.
    pub fn print_response<W: Write>(
        &self,
        out: &mut BufWriter<W>
    ) -> NetResult<()> {
        if let Some(res) = self.res.as_ref() {
            if self.output.include_separator() {
                writeln!(out)?;
            }

            self.output.print_status_line(res, out)?;
            self.output.print_res_headers(res, out)?;

            let is_head_route = self
                .req
                .as_ref()
                .map(|req| req.route().is_head())
                .unwrap_or(false);

            if !is_head_route {
                self.output.print_res_body(res, out)?;
            }
        }

        Ok(())
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
        if self.req.is_some() {
            self.print_request(out)?;
        }

        // Handle response output.
        if self.res.is_some() {
            self.print_response(out)?;
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
        match self.req.as_mut() {
            None => Err(NetError::NoRequest),
            Some(req) => match self.conn.as_mut() {
                None => Err(NetError::NotConnected),
                Some(conn) => conn.send_request(req),
            },
        }
    }

    /// Writes an HTTP `Response` to a `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::send_response` fails.
    pub fn send_response(&mut self) -> NetResult<()> {
        match self.res.as_mut() {
            None => Err(NetError::NoResponse),
            Some(res) => match self.conn.as_mut() {
                None => Err(NetError::NotConnected),
                Some(conn) => conn.send_response(res),
            },
        }
    }

    /// Reads and parse an HTTP `Request` from the contained `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if `Connection::recv_request` fails.
    pub fn recv_request(&mut self) -> NetResult<()> {
        let req = self
            .conn
            .as_mut()
            .ok_or(NetError::NotConnected)
            .and_then(|conn| conn.recv_request())?;

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
            .and_then(|conn| conn.recv_response())?;

        self.res = Some(res);

        Ok(())
    }
}
