use std::borrow::ToOwned;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::ErrorKind as IoErrorKind;
use std::io::Result as IoResult;
use std::net::{TcpStream, ToSocketAddrs};

use crate::consts::{CONTENT_LENGTH, CONTENT_TYPE};
use crate::{
    Body, Connection, HeaderName, HeaderValue, Headers, Method, NetError,
    NetResult, ParseErrorKind, RequestLine, Response, Request, Version,
};

/// An HTTP request builder object.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ClientBuilder<A>
where
    A: ToSocketAddrs
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
    A: ToSocketAddrs
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
    A: ToSocketAddrs
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

	/// Sets the request body and adds Content-Type and Content-Length
    /// headers.
    #[must_use]
	pub fn body(mut self, data: &[u8]) -> Self {
		if data.is_empty() {
            return self;
        }

        self.headers.insert(CONTENT_LENGTH, data.len().into());
        self.headers.insert(CONTENT_TYPE, b"text/plain"[..].into());
        self.body = Body::Text(String::from_utf8_lossy(data).to_string());
        self
	}

    /// Builds and returns a new `Client` instance.
    pub fn build(self) -> NetResult<Client> {
        let conn = {
			if let Some(addr) = self.addr.as_ref() {
				let stream = TcpStream::connect(addr)?;
                Connection::try_from(stream)?
            } else if self.ip.is_some() && self.port.is_some() {
                let ip = self.ip.as_ref().unwrap();
                let port = self.port.as_ref().unwrap();

                let addr = format!("{ip}:{port}");
                let stream = TcpStream::connect(addr)?;

                Connection::try_from(stream)?
            } else {
                return Err(IoErrorKind::InvalidInput.into());
            }
        };

        let path = self.path.as_ref()
            .map_or_else(
                || String::from("/"),
                ToOwned::to_owned);

        let request_line = RequestLine::new(self.method, path, self.version);
        let headers = self.headers.clone();
        let body = self.body;
        let conn = Some(conn);

        let req = Some(Request { request_line, headers, body, conn });
        let res = None;

        Ok(Client { req, res })
    }

    /// Sends an HTTP request and then returns a `Client` instance.
    pub fn send(self) -> IoResult<Client> {
        let mut client = self.build()?;
        client.send()?;
        Ok(client)
    }
}

/// An HTTP client that can send and receive messages with a remote server.
#[derive(Debug)]
pub struct Client {
	pub req: Option<Request>,
	pub res: Option<Response>,
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
        A: ToSocketAddrs
    {
        ClientBuilder::new()
    }
    

    /// Parses a string slice into a host address and a URI path.
    pub fn parse_uri(uri: &str) -> NetResult<(String, String)> {
        let uri = uri.trim();

        if let Some((scheme, rest)) = uri.split_once("://") {
            // If "://" is present, we expect a URI like "http://httpbin.org".
            if scheme.is_empty() || rest.is_empty() {
                return Err(ParseErrorKind::Uri.into());
            }

            match scheme {
                "http" => match rest.split_once('/') {
                    // Next "/" after the scheme, if present, starts the
                    // path segment.
                    Some((addr, path)) if path.is_empty() && addr.contains(':') => {
                        // Example: http://httpbin.org:80/
                        Ok((addr.to_string(), String::from("/")))
                    },
                    Some((addr, path)) if path.is_empty() => {
                        // Example: http://httpbin.org/
                        Ok((format!("{addr}:80"), String::from("/")))
                    },
                    Some((addr, path)) if addr.contains(':') => {
                        // Example: http://httpbin.org:80/json
                        Ok((addr.to_string(), format!("/{path}")))
                    },
                    Some((addr, path)) => {
                        // Example: http://httpbin.org/json
                        Ok((format!("{addr}:80"), format!("/{path}")))
                    },
                    None if rest.contains(':') => {
                        // Example: http://httpbin.org:80
                        Ok((rest.to_string(), String::from("/")))
                    },
                    None => {
                        // Example: http://httpbin.org
                        Ok((format!("{rest}:80"), String::from("/")))
                    },
                },
                "https" => Err(NetError::HttpsNotImplemented),
                _ => Err(ParseErrorKind::Uri)?,
            }
        } else if let Some((addr, path)) = uri.split_once('/') {
            if addr.is_empty() {
                return Err(ParseErrorKind::Uri)?;
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
    pub fn send(&mut self) -> NetResult<()> {
        self.req
            .as_mut()
            .ok_or_else(|| IoErrorKind::NotConnected.into())
            .and_then(|req| req.send())
    }

    /// Receives an HTTP response from the remote host.
    pub fn recv(&mut self, conn: &mut Connection) -> NetResult<()> {
        Response::recv(conn).and_then(|res| {
            self.res = Some(res);
            Ok(())
        })
    }
}
