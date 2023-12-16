use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::ErrorKind as IoErrorKind;
use std::net::{TcpStream, ToSocketAddrs};

use crate::consts::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, HOST, USER_AGENT};
use crate::{
    Body, HeaderName, HeaderValue, Headers, Method, NetError, NetReader, NetResult, NetWriter,
    ParseErrorKind, Request, RequestLine, Response, Version,
};

/// An HTTP request builder object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientBuilder<A>
where
    A: ToSocketAddrs,
{
    pub method: Method,
    pub ip: Option<String>,
    pub port: Option<u16>,
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
            ip: None,
            port: None,
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

    /// Sets the remote host's IP address.
    #[must_use]
    pub fn ip(mut self, ip: &str) -> Self {
        self.ip = Some(ip.to_string());
        self
    }

    /// Sets the remote host's port.
    #[must_use]
    pub const fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
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
    pub fn insert_header(mut self, name: HeaderName, value: HeaderValue) -> Self {
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
            self.headers.insert_content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = body;
            self.headers.insert_content_length(self.body.len());
            self.headers.insert_content_type(content_type);
        }

        self
    }

    /// Sets a text request body and sets the Content-Type and Content-Length
    /// headers.
    #[must_use]
    pub fn text(mut self, text: &str) -> Self {
        if text.is_empty() {
            self.headers.insert_content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = Body::Text(text.into());
            self.headers.insert_content_length(self.body.len());
            self.headers
                .insert_content_type("text/plain; charset=utf-8");
        }

        self
    }

    /// Sets a HTML request body and sets the Content-Type and Content-Length
    /// headers.
    #[must_use]
    pub fn html(mut self, html: &str) -> Self {
        if html.is_empty() {
            self.headers.insert_content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = Body::Html(html.into());
            self.headers.insert_content_length(self.body.len());
            self.headers.insert_content_type("text/html; charset=utf-8");
        }

        self
    }

    /// Sets a JSON request body and sets the Content-Type and Content-Length
    /// headers.
    #[must_use]
    pub fn json(mut self, json: &str) -> Self {
        if json.is_empty() {
            self.headers.insert_content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = Body::Json(json.into());
            self.headers.insert_content_length(self.body.len());
            self.headers.insert_content_type("application/json");
        }

        self
    }

    /// Sets a request body comprised of bytes and sets the Content-Type and
    /// Content-Length headers.
    #[must_use]
    pub fn bytes(mut self, bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            self.headers.insert_content_length(0);
            self.body = Body::Empty;
        } else {
            self.body = Body::Bytes(bytes.to_vec());
            self.headers.insert_content_type("application/octet-stream");
        }

        self
    }

    /// Builds and returns a new `Client` instance.
    #[allow(clippy::missing_errors_doc)]
    pub fn build(mut self) -> NetResult<Client> {
        let stream = match self.addr.as_ref() {
            Some(addr) => TcpStream::connect(addr)?,
            None => match (self.ip.as_ref(), self.port.as_ref()) {
                (Some(ip), Some(port)) => TcpStream::connect((ip.as_str(), *port))?,
                (_, _) => return Err(IoErrorKind::InvalidInput.into()),
            },
        };

        let reader = NetReader::from(stream.try_clone()?);
        let writer = NetWriter::from(stream);

        if !self.headers.contains(&ACCEPT) {
            self.headers.insert_accept("*/*");
        }

        if !self.headers.contains(&CONTENT_LENGTH) {
            self.headers.insert_content_length(self.body.len());
        }

        if !self.headers.contains(&CONTENT_TYPE) && !self.body.is_empty() {
            self.headers.insert_content_type("text/plain");
        }

        if !self.headers.contains(&HOST) {
            let remote = reader.get_ref().peer_addr()?;
            self.headers.insert_host(remote.ip(), remote.port());
        }

        if !self.headers.contains(&USER_AGENT) {
            self.headers.insert_user_agent();
        }

        let path = self
            .path
            .as_ref()
            .map_or_else(|| String::from("/"), ToString::to_string);

        let request_line = RequestLine::new(self.method, path, self.version);

        let req = Some(Request {
            request_line,
            headers: self.headers,
            body: self.body,
        });

        let res = None;

        Ok(Client {
            req,
            res,
            reader,
            writer,
        })
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

/// An HTTP client that can send and receive messages with a remote server.
#[derive(Debug)]
pub struct Client {
    pub req: Option<Request>,
    pub res: Option<Response>,
    pub reader: NetReader,
    pub writer: NetWriter,
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

    /// Parses a string slice into a host address and a URI path.
    #[allow(clippy::missing_errors_doc)]
    pub fn parse_uri(uri: &str) -> NetResult<(String, String)> {
        let uri = uri.trim();

        if let Some((scheme, rest)) = uri.split_once("://") {
            // If "://" is present, we expect a URI like "http://httpbin.org".
            if scheme.is_empty() || rest.is_empty() {
                return Err(ParseErrorKind::Path.into());
            }

            match scheme {
                "http" => match rest.split_once('/') {
                    // Next "/" after the scheme, if present, starts the
                    // path segment.
                    Some((addr, path)) if path.is_empty() && addr.contains(':') => {
                        // Example: http://httpbin.org:80/
                        Ok((addr.to_string(), String::from("/")))
                    }
                    Some((addr, path)) if path.is_empty() => {
                        // Example: http://httpbin.org/
                        Ok((format!("{addr}:80"), String::from("/")))
                    }
                    Some((addr, path)) if addr.contains(':') => {
                        // Example: http://httpbin.org:80/json
                        Ok((addr.to_string(), format!("/{path}")))
                    }
                    Some((addr, path)) => {
                        // Example: http://httpbin.org/json
                        Ok((format!("{addr}:80"), format!("/{path}")))
                    }
                    None if rest.contains(':') => {
                        // Example: http://httpbin.org:80
                        Ok((rest.to_string(), String::from("/")))
                    }
                    None => {
                        // Example: http://httpbin.org
                        Ok((format!("{rest}:80"), String::from("/")))
                    }
                },
                "https" => Err(NetError::HttpsNotImplemented),
                _ => Err(ParseErrorKind::Path)?,
            }
        } else if let Some((addr, path)) = uri.split_once('/') {
            if addr.is_empty() {
                return Err(ParseErrorKind::Path)?;
            }

            let addr = if addr.contains(':') {
                addr.to_string()
            } else {
                format!("{addr}:80")
            };

            let path = if path.is_empty() {
                String::from("/")
            } else {
                format!("/{path}")
            };

            Ok((addr, path))
        } else if uri.contains(':') {
            Ok((uri.to_string(), String::from("/")))
        } else {
            Ok((format!("{uri}:80"), String::from("/")))
        }
    }

    /// Sends an HTTP request to a remote host.
    #[allow(clippy::missing_errors_doc)]
    pub fn send(&mut self) -> NetResult<()> {
        self.req
            .as_mut()
            .ok_or(NetError::IoError(IoErrorKind::NotConnected))
            .and_then(|req| req.send(&mut self.writer))?;

        Ok(())
    }

    /// Receives an HTTP response from the remote host.
    #[allow(clippy::missing_errors_doc)]
    pub fn recv(&mut self) {
        self.res = self.reader.recv_response().ok();
    }
}
