use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{self, BufRead, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::str::FromStr;

use crate::{
    Body, Connection, Headers, Method, NetError, NetResult, Request,
    Response, Style, UriPath, Version,
};
use crate::headers::names::DATE;
use crate::style::colors::{GREEN, ORANGE, RESET, BLUE};
use crate::utils;

/// An HTTP client builder object.
#[derive(Debug)]
pub struct ClientBuilder {
    pub do_send: bool,
    pub do_debug: bool,
    pub no_dates: bool,
    pub style: Option<Style>,
    pub method: Option<Method>,
    pub path: Option<UriPath>,
    pub version: Option<Version>,
    pub headers: Option<Headers>,
    pub body: Option<Body>,
    pub conn: Option<NetResult<Connection>>,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            do_send: true,
            do_debug: false,
            no_dates: false,
            style: None,
            method: None,
            path: None,
            version: None,
            headers: None,
            body: None,
            conn: None
        }
    }
}

impl ClientBuilder {
    /// Returns a new `ClientBuilder` instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to send the request.
    pub fn do_send(&mut self, do_send: bool) -> &mut Self {
        self.do_send = do_send;
        self
    }

    /// Enables debug printing.
    pub fn do_debug(&mut self, do_debug: bool) -> &mut Self {
        self.do_debug = do_debug;
        self
    }

    /// Sets whether to print Date headers.
    pub fn no_dates(&mut self, no_dates: bool) -> &mut Self {
        self.no_dates = no_dates;
        self
    }

    /// Sets the HTTP method.
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = Some(method);
        self
    }

    /// Sets the URI path.
    pub fn path(&mut self, path: UriPath) -> &mut Self {
        self.path = Some(path);
        self
    }

    /// Sets the HTTP version.
    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    /// Sets the remote host's address.
    pub fn addr<A: ToSocketAddrs>(&mut self, addr: A) -> &mut Self {
        let conn_result = TcpStream::connect(addr)
            .map_err(|e| NetError::Io(e.kind()))
            .and_then(Connection::try_from);

        self.conn = Some(conn_result);
        self
    }

    /// Inserts a request header.
    pub fn header(&mut self, name: &str, value: &str) -> &mut Self {
        if let Some(headers) = self.headers.as_mut() {
            headers.header(name, value);
        } else {
            let mut headers = Headers::default();
            headers.header(name, value);
            self.headers = Some(headers);
        }

        self
    }

    /// Sets the request headers.
    pub fn headers(&mut self, mut headers: Headers) -> &mut Self {
        match self.headers.as_mut() {
            Some(hdrs) => hdrs.append(&mut headers),
            None => self.headers = Some(headers),
        }

        self
    }

    /// Sets the request body.
    pub fn body(&mut self, body: Body) -> &mut Self {
        if !body.is_empty() {
            self.body = Some(body);
        }

        self
    }

    /// Sets the output style.
    pub fn style(&mut self, style: Style) -> &mut Self {
        self.style = Some(style);
        self
    }

    /// Builds and returns a new `Client`.
    ///
    /// # Errors
    /// 
    /// Returns an error if establishing a TCP connection fails.
    pub fn build(&mut self) -> NetResult<Client> {
        let conn = match self.conn.take() {
            Some(Ok(conn)) => conn,
            Some(Err(e)) => return Err(e),
            None => return Err(NetError::NotConnected),
        };

        // `Request::builder` sets default request headers if not present.
        let req = Request::builder()
            .method(self.method.take().unwrap_or_default())
            .path(self.path.take().unwrap_or_default())
            .version(self.version.take().unwrap_or_default())
            .headers(self.headers.take().unwrap_or_default())
            .body(self.body.take().unwrap_or_default())
            .build();

        Ok(Client {
            do_send: self.do_send,
            do_debug: self.do_debug,
            no_dates: self.no_dates,
            style: self.style.take().unwrap_or_default(),
            req: Some(req),
            res: None,
            conn: Some(conn)
        })
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
pub struct Client {
    pub do_send: bool,
    pub do_debug: bool,
    pub no_dates: bool,
    pub style: Style,
    pub req: Option<Request>,
    pub res: Option<Response>,
    pub conn: Option<Connection>,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            do_send: true,
            do_debug: false,
            no_dates: false,
            style: Style::default(),
            req: None,
            res: None,
            conn: None
        }
    }
}

impl PartialEq for Client {
    fn eq(&self, other: &Self) -> bool {
        self.do_send == other.do_send
            && self.do_debug == other.do_debug
            && self.no_dates == other.no_dates
            && self.style == other.style
            && self.req == other.req
            && self.res == other.res
            && self.conn.is_some() == other.conn.is_some()
    }
}

impl Eq for Client {}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if let Some(req) = self.req.as_ref() {
            writeln!(f, "{req}")?;
        }

        if self.req.is_some() && self.res.is_some() {
            writeln!(f)?;
        }

        if let Some(res) = self.res.as_ref() {
            writeln!(f, "{res}")?;
        }

        Ok(())
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "Client {{")?;
        writeln!(f, "    do_send: {:?},", self.do_send)?;
        writeln!(f, "    do_debug: {:?},", self.do_debug)?;
        writeln!(f, "    no_dates: {:?},", self.no_dates)?;
        writeln!(f, "    style: Style {{")?;
        writeln!(f, "        req: {:?},", &self.style.req)?;
        writeln!(f, "        res: {:?},", &self.style.res)?;
        writeln!(f, "    }},")?;

        if let Some(req) = self.req.as_ref() {
            writeln!(f, "    req: Some(Request {{")?;
            writeln!(f, "        method: {:?},", &req.method)?;
            writeln!(f, "        path: {:?},", &req.path)?;
            writeln!(f, "        version: {:?},", &req.version)?;
            writeln!(f, "        headers: Headers(")?;
            for (name, value) in &req.headers.0 {
                write!(f, "            ")?;
                writeln!(f, "{name:?}: {value:?},")?;
            }
            writeln!(f, "        ),")?;
            if req.body.is_empty() {
                writeln!(f, "        body: Body::Empty")?;
            } else if req.body.is_printable() {
                writeln!(f, "        body: {:?}", &req.body)?;
            } else {
                writeln!(f, "        body: Body {{ ... }}")?;
            }
            writeln!(f, "    }}),")?;
        } else {
            writeln!(f, "    req: None,")?;
        }

        if let Some(res) = self.res.as_ref() {
            writeln!(f, "    req: Some(Response {{")?;
            writeln!(f, "        version: {:?},", &res.version)?;
            writeln!(f, "        status: {:?},", &res.status)?;
            writeln!(f, "        headers: Headers(")?;
            for (name, value) in &res.headers.0 {
                write!(f, "            ")?;
                writeln!(f, "{name:?}: {value:?},")?;
            }
            writeln!(f, "        ),")?;
            if res.body.is_empty() {
                writeln!(f, "        body: Body::Empty")?;
            } else if res.body.is_printable() {
                writeln!(f, "        body: {:?}", &res.body)?;
            } else {
                writeln!(f, "        body: Body {{ ... }}")?;
            }
            writeln!(f, "    }}),")?;
        } else {
            writeln!(f, "    res: None,")?;
        }

        if self.conn.is_some() {
            writeln!(f, "    conn: Some(Connection {{ ... }})")?;
        } else {
            writeln!(f, "    conn: None")?;
        }

        write!(f, "}}")?;
        Ok(())
    }
}

impl Client {
    /// Returns a new `ClientBuilder` instance.
    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Returns a new `Client` from the given HTTP method and URI.
    ///
    /// # Errors
    ///
    /// Returns an error `TcpStream::connect` is unable to connect to the the
    /// given `uri`.
    pub fn new(method: Method, uri: &str) -> NetResult<Self> {
        let (addr, path) = utils::parse_uri(uri)?;

        Self::builder()
            .method(method)
            .addr(&addr)
            .path(path.into())
            .build()
    }

    /// Sends an HTTP request to the given URI using the provided HTTP method,
    /// returning the `Client` instance.
    /// 
    /// # Errors
    /// 
    /// Returns an error `TcpStream::connect` is unable to connect to the the
    /// given `uri` or if sending the request fails.
    pub fn send(method: Method, uri: &str) -> NetResult<Self> {
        let (addr, path) = utils::parse_uri(uri)?;

        Self::builder()
            .method(method)
            .addr(&addr)
            .path(path.into())
            .send()
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

    /// Removes Date header field entries from requests and responses.
    pub fn remove_date_headers(&mut self) {
        if let Some(req) = self.req.as_mut() {
            req.headers.remove(&DATE);
        }

        if let Some(res) = self.res.as_mut() {
            res.headers.remove(&DATE);
        }
    }

    /// Returns true if a component of both the request and the response is
    /// printed.
    #[must_use]
    pub const fn include_separator(&self) -> bool {
        self.req.is_some()
            && self.style.req.is_printed()
            && self.res.is_some()
            && self.style.res.is_printed()
            && !self.style.is_minimal()
    }

    /// Prints the `RequestLine` if appropriate for the `Style`.
    pub fn print_request_line(&self, req: &Request) {
        if self.style.req.is_plain_first_line() {
            println!("{}", &req.request_line_to_plain_string().trim_end());
        } else if self.style.req.is_color_first_line() {
            println!("{}", &req.request_line_to_color_string().trim_end());
        }
    }

    /// Prints the status line if appropriate for the `Style`.
    pub fn print_status_line(&self, res: &Response) {
        if self.style.res.is_plain_first_line() {
            println!("{}", &res.status_line_to_plain_string().trim_end());
        } else if self.style.res.is_color_first_line() {
            println!("{}", &res.status_line_to_color_string().trim_end());
        }
    }

    /// Prints the request `Headers` if appropriate for the `Style`.
    pub fn print_req_headers(&self, req: &Request) {
        if self.style.req.is_plain_headers() {
            println!("{}", &req.headers.to_string().trim_end());
        } else if self.style.req.is_color_headers() {
            println!("{}", &req.headers.to_color_string().trim_end());
        }
    }

    /// Prints the response `Headers` if appropriate for the `Style`.
    pub fn print_res_headers(&self, res: &Response) {
        if self.style.res.is_plain_headers() {
            println!("{}", &res.headers.to_string().trim_end());
        } else if self.style.res.is_color_headers() {
            println!("{}", &res.headers.to_color_string().trim_end());
        }
    }

    /// Prints the request `Body` if appropriate for the `Style`.
    pub fn print_req_body(&self, req: &Request) {
        if self.style.req.is_body() && req.body.is_printable() {
            println!("{}", req.body.to_string().trim_end());
        }
    }

    /// Prints the response `Body` if appropriate for the `Style`.
    pub fn print_res_body(&self, res: &Response) {
        if self.style.res.is_body() && res.body.is_printable() {
            println!("{}", res.body.to_string().trim_end());
        }
    }

    /// Prints the request and the response to stdout based on the `Style`.
    pub fn print(&mut self) {
        let mut is_not_head = true;

        // Remove Date headers based on output style.
        if self.no_dates {
            self.remove_date_headers();
        }

        // Handle request output.
        if let Some(req) = self.req.as_ref() {
            self.print_request_line(req);
            self.print_req_headers(req);
            self.print_req_body(req);

            is_not_head = !req.route().is_head();
        }

        if self.include_separator() {
            println!();
        }

        // Handle response output.
        if let Some(res) = self.res.as_ref() {
            self.print_status_line(res);
            self.print_res_headers(res);

            if is_not_head {
                self.print_res_body(res);
            }
        }

        println!();
    }

    /// Reads and parses a URI from stdin.
    ///
    /// # Errors
    ///
    /// Returns an error if a problem occurs while reading or writing to the
    /// terminal.
    pub fn get_uri(line: &mut String) -> NetResult<(String, UriPath)> {
        let mut stdout = io::stdout().lock();

        writeln!(
            &mut stdout,
            "Note: press \"Enter\" to skip optional fields.\n"
        )?;

        loop {
            write!(&mut stdout, "{BLUE}URI:{RESET} ")?;
            stdout.flush()?;

            line.clear();
            io::stdin().lock().read_line(line)?;

            let trimmed = line.trim();

            if trimmed.is_empty() {
                continue;
            }

            let Ok((addr, path)) = utils::parse_uri(trimmed) else {
                writeln!(&mut stdout, "{ORANGE}Invalid URI.{RESET}")?;
                continue;
            };

            return Ok((addr, path.into()));
        }
    }

    /// Reads and parses a `Method` from stdin.
    ///
    /// # Errors
    ///
    /// Returns an error if a problem occurs while reading or writing to the
    /// terminal.
    pub fn get_method(line: &mut String) -> NetResult<Method> {
        let mut stdout = io::stdout().lock();

        write!(
            &mut stdout,
            "{BLUE}Method{RESET} (optional): "
        )?;
        stdout.flush()?;

        line.clear();
        io::stdin().lock().read_line(line)?;

        let trimmed = line.trim();

        if trimmed.is_empty() {
            Ok(Method::Get)
        } else {
            let uppercase = trimmed.to_ascii_uppercase();
            let method = Method::from_str(uppercase.as_str())?;
            Ok(method)
        }
    }

    /// Reads and parses zero or more `Header` values from stdin.
    ///
    /// # Errors
    ///
    /// Returns an error if a problem occurs while reading or writing to the
    /// terminal.
    pub fn get_headers(line: &mut String) -> NetResult<Headers> {
        let mut headers = Headers::new();

        let mut stdout = io::stdout().lock();

        loop {
            write!(
                &mut stdout,
                "{BLUE}Header{RESET} (optional; name:value): "
            )?;
            stdout.flush()?;

            line.clear();
            io::stdin().lock().read_line(line)?;

            let trimmed = line.trim();

            if trimmed.is_empty() {
                return Ok(headers);
            }

            if let Some((name, value)) = trimmed.split_once(':') {
                headers.header(name, value);

                writeln!(
                    &mut stdout,
                    "{GREEN}{} header added.{RESET}",
                    utils::to_titlecase(name.as_bytes())
                )?;

                continue;
            }

            writeln!(
                &mut stdout,
                "{ORANGE}Invalid input.\n\
                Press \"Enter\" to finish adding headers.{RESET}"
            )?;
        }
    }

    /// Reads and parses a `Body` from stdin.
    ///
    /// # Errors
    ///
    /// Returns an error if a problem occurs while reading or writing to the
    /// terminal.
    pub fn get_body(line: &mut String) -> NetResult<Body> {
        let mut stdout = io::stdout().lock();

        write!(&mut stdout, "{BLUE}Body{RESET} (optional): ")?;
        stdout.flush()?;

        line.clear();
        io::stdin().lock().read_line(line)?;

        let trimmed = line.trim();

        if trimmed.is_empty() {
            Ok(Body::Empty)
        } else {
            Ok(String::from(trimmed).into())
        }
    }

    /// Handles prompting the user for input to build a `Request`.
    ///
    /// # Errors
    ///
    /// Returns an error if a problem occurs while reading or writing to the
    /// terminal or if building the `Request` fails.
    pub fn get_request_from_user(&mut self) -> NetResult<()> {
        let mut line = String::with_capacity(1024);

        let (addr, path) = Self::get_uri(&mut line)?;
        let method = Self::get_method(&mut line)?;
        let headers = Self::get_headers(&mut line)?;
        let body = Self::get_body(&mut line)?;

        self.req = Some(Request::builder()
            .method(method)
            .path(path)
            .headers(headers)
            .body(body)
            .build());

        self.conn = Some(TcpStream::connect(&addr)
            .map_err(|e| NetError::Io(e.kind()))
            .and_then(Connection::try_from)?);

        Ok(())
    }
}
