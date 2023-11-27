use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::ErrorKind as IoErrorKind;
use std::net::{IpAddr, SocketAddr};
use std::string::ToString;

use crate::consts::{CONNECTION, CONTENT_TYPE};
use crate::{
    Body, Connection, HeaderName, HeaderValue, Headers, Method, NetReader,
    NetResult, Request, Status, Target, Version,
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
    pub conn: Option<Connection>,
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
            .field("conn", &self.conn)
			.field("status_line", &self.status_line)
			.field("headers", &self.headers)
            .field("body", &self.body)
			.finish()
	}
}

impl Response {
    /// Returns a new `Response` object.
    pub fn new(
        code: u16,
        target: &Target,
        req: &mut Request
    ) -> NetResult<Self> {
        let mut res = Self {
            status_line: StatusLine::new(Version::OneDotOne, Status(code)),
            headers: Headers::new(),
            body: Body::Empty,
            conn: req.conn.take()
        };

        res.headers.insert_cache_control("no-cache");

        match target {
            Target::Text(s) => {
                res.headers.insert_content_length(s.len());
                res.headers.insert_content_type("text/plain; charset=utf-8");
                res.body = Body::Text((*s).into());
            },
            Target::Html(s) => {
                res.headers.insert_content_length(s.len());
                res.headers.insert_content_type("text/html; charset=utf-8");
                res.body = Body::Html((*s).into());
            },
            Target::Json(s) => {
                res.headers.insert_content_length(s.len());
                res.headers.insert_content_type("application/json");
                res.body = Body::Json((*s).into());
            },
            Target::Xml(s) => {
                res.headers.insert_content_length(s.len());
                res.headers.insert_content_type("application/xml");
                res.body = Body::Xml((*s).into());
            },
            Target::File(ref fpath) => {
                let content = fs::read(fpath)?;
                let cont_type = HeaderValue::infer_content_type(fpath);
                res.headers.insert(CONTENT_TYPE, cont_type);
                res.headers.insert_content_length(content.len());
                res.headers.insert_cache_control("max-age=604800");
                res.body = Body::Bytes(content);
            },
            Target::Favicon(ref fpath) => {
                let content = fs::read(fpath)?;
                res.headers.insert_content_type("image/x-icon");
                res.headers.insert_content_length(content.len());
                res.headers.insert_cache_control("max-age=604800");
                res.body = Body::Favicon(content);
            },
            Target::Fn(handler) => {
                // Call handler to perform an action.
                (handler)(req, &res);
            },
            Target::FnMut(handler) => {
                // Call handler to update the response.
                (handler.lock().unwrap())(req, &mut res);

                if !res.body.is_empty() {
                    res.headers.insert_content_length(res.body.len());
                    res.headers.insert_content_type("text/plain; charset=utf-8");
                }
            },
            // Target::FnOnce(handler) => {
            //     // Handler returns a Body instance.
            //     let body = (handler)();
            //     let contype = body.as_content_type();

            //     if let Some((contype_name, contype_value)) = contype {
            //         res.headers.insert_content_length(body.len());
            //         res.headers.insert(contype_name, contype_value);
            //         res.body = body.clone();
            //     }
            // },
            _ => {},
        }

        if req.method() == Method::Head {
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
        self.conn.as_ref().map(|sock| sock.remote_addr)
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
        self.conn.as_ref().map(|sock| sock.local_addr)
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
    ///
    /// Presence of a response body depends upon the request method and the
    /// response status code.
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
        let Some(conn) = self.conn.as_mut() else {
            return Err(IoErrorKind::NotConnected.into());
        };

        let mut writer = conn.writer.try_clone()?;
        writer.send_response(self)
    }

    /// Receives an HTTP response from a remote server.
    pub fn recv(mut reader: NetReader) -> NetResult<Response> {
        reader.recv_response()
    }
}
