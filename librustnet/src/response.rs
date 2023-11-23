use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::{
    ErrorKind as IoErrorKind, Result as IoResult, Write,
};

use crate::consts::{
    CONNECTION, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, DEFAULT_NAME,
    SERVER,
};
use crate::{
    HeaderName, HeaderValue, HeadersMap, Method, NetResult,
    Connection, NetError, Request, Status, Target, Version, Resolved,
};

/// Represents the components of an HTTP response.
pub struct Response {
    pub conn: Option<Connection>,
    pub method: Method,
    pub path: String,
    pub version: Version,
    pub status: Status,
    pub headers: HeadersMap,
    pub body: Option<Vec<u8>>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            conn: None,
            method: Method::default(),
            path: String::from("/"),
            version: Version::default(),
            status: Status(200),
            headers: BTreeMap::new(),
            body: None,
        }
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // The response status line.
        writeln!(f, "{}", self.status_line())?;

        // The response headers.
        if !self.headers.is_empty() {
            for (name, value) in &self.headers {
                writeln!(f, "{name}: {value}")?;
            }
        }

        // The response body.
		if let Some(body) = self.body.as_ref() {
			if !body.is_empty() && self.body_is_printable() {
				let body = String::from_utf8_lossy(body);
				write!(f, "\n{body}")?;
			}
		}

		Ok(())
    }
}

impl Debug for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		let mut dbg = f.debug_struct("Response");

		let dbg = dbg.field("method", &self.method)
			.field("path", &self.path)
			.field("version", &self.version)
			.field("status", &self.status)
			.field("headers", &self.headers);

		if self.body.is_none() {
			dbg.field("body", &self.body).finish()
		} else if !self.body_is_permitted() || !self.body_is_printable() {
			dbg.field("body", &"...").finish()
		} else {
			let body = self.body.as_ref().map(|b| String::from_utf8_lossy(b));
			dbg.field("body", &body).finish()
		}
	}
}

impl Response {
    /// Parses a `Response` object from a `Request`.
    pub fn from_request(mut req: Request, resolved: &Resolved) -> NetResult<Self> {
        let path = req.path.clone();
        let version = req.version;

        let method = resolved.method;
        let status = resolved.status;

        let mut headers = BTreeMap::new();

        let body = match resolved.target() {
            Target::File(ref f) => {
                let content = fs::read(f)?;

				if content.is_empty() {
					None
				} else {
					let contype = HeaderValue::infer_content_type(f);
					headers.insert(CONTENT_TYPE, contype);
					headers.insert(CONTENT_LENGTH, content.len().into());
					
					if method == Method::Head {
						None
					} else {
						Some(content)
					}
				}
            },
            Target::Text(ref s) => Some(s.clone().into_bytes()),
            Target::Bytes(ref b) => Some(b.to_owned()),
            Target::Empty => None,
        };

        let conn = req.conn.take();

        Ok(Self {
            conn,
            method,
            path,
            version,
            status,
            headers,
            body,
        })
    }

    /// Returns the HTTP method of the original request.
    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.method
    }

    /// Returns the URI path to the target resource.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the protocol version.
    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    /// Returns the response's `Status` value.
    #[must_use]
    pub const fn status(&self) -> &Status {
        &self.status
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

    /// Returns a map of the response's headers.
    #[must_use]
    pub const fn headers(&self) -> &HeadersMap {
        &self.headers
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains_key(name)
    }

    /// Adds or modifies the header field represented by `HeaderName`.
    pub fn set_header(&mut self, name: HeaderName, val: HeaderValue) {
        if self.has_header(&name) {
            self.headers.entry(name).and_modify(|v| *v = val);
        } else {
            self.headers.insert(name, val);
        }
    }

    /// Returns the header value for the given `HeaderName`, if present.
    #[must_use]
    pub fn header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Returns all of the response headers as a String.
	#[must_use]
    pub fn headers_to_string(&self) -> String {
        if self.headers.is_empty() {
            String::new()
        } else {
            self.headers
                .iter()
                .fold(String::new(), |mut acc, (name, value)| {
                    let header = format!("{name}: {value}\n");
                    acc.push_str(&header);
                    acc
                })
        }
    }

    /// Returns true if a response body is allowed.
    ///
    /// Presence of a response body depends upon the request method and the
    /// response status code.
    #[must_use]
    pub fn body_is_permitted(&self) -> bool {
        match self.status_code() {
            // 1xx (Informational), 204 (No Content), and 304 (Not Modified).
            100..=199 | 204 | 304 => false,
            // CONNECT responses with a 2xx (Success) status.
            200..=299 if self.method == Method::Connect => false,
            // HEAD responses.
            _ if self.method == Method::Head => false,
            _ => true,
        }
    }

    /// Returns an optional reference to the message body, if present.
    #[must_use]
    pub const fn body(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }

	/// Returns true if the body is unencoded and has a text or application
	/// Content-Type header.
	#[must_use]
    pub fn body_is_printable(&self) -> bool {
        if self.has_header(&CONTENT_ENCODING)
			|| !self.has_header(&CONTENT_TYPE)
        {
            return false;
        }

        self.header(&CONTENT_TYPE).map_or(false, |ct| {
            let ct = ct.to_string();
			ct.contains("text") || ct.contains("application")
        })
	}

    /// Returns the response body as a copy-on-write string.
    #[must_use]
    pub fn body_to_string(&self) -> Cow<'_, str> {
        if let Some(body) = self.body.as_ref() {
            if !body.is_empty() && self.body_is_printable() {
                return String::from_utf8_lossy(body);
            }
        }

        String::new().into()
    }

    /// Returns a String representation of the response's status line.
    #[must_use]
    pub fn status_line(&self) -> String {
        format!("{} {}", &self.version, &self.status)
    }

    /// Writes the response's status line to a stream.
    pub fn write_status_line(&mut self) -> IoResult<()> {
        let writer = self
            .conn
            .as_mut()
            .map(|conn| conn.writer.by_ref())
            .ok_or(NetError::WriteError(IoErrorKind::NotConnected))?;

        write!(writer, "{} {}\r\n", &self.version, &self.status)?;
        writer.flush()?;
        Ok(())
    }

    /// Writes the response's headers to a stream.
    pub fn write_headers(&mut self) -> IoResult<()> {
        self.set_header(SERVER, HeaderValue::from(DEFAULT_NAME));

        let writer = self
            .conn
            .as_mut()
            .map(|conn| conn.writer.by_ref())
            .ok_or(NetError::WriteError(IoErrorKind::NotConnected))?;

        if !self.headers.is_empty() {
            self.headers.iter().for_each(|(name, value)| {
                write!(writer, "{name}: ").unwrap();
                writer.write_all(value.as_bytes()).unwrap();
                writer.write_all(b"\r\n").unwrap();
            });
        }

        // Mark the end of the headers section.
        writer.write_all(b"\r\n")?;
        writer.flush()?;
        Ok(())
    }

    /// Writes the response's body to a stream, if applicable.
    pub fn write_body(&mut self) -> IoResult<()> {
        let body_is_permitted = self.body_is_permitted();

        let writer = self
            .conn
            .as_mut()
            .map(|conn| conn.writer.by_ref())
            .ok_or(NetError::WriteError(IoErrorKind::NotConnected))?;

        if let Some(body) = self.body.as_ref() {
			if !body.is_empty() && body_is_permitted {
				writer.write_all(body)?;
			}
		}

        writer.flush()?;
        Ok(())
    }

    /// Writes the `Response` to the underlying TCP connection that is
    /// established with the remote client.
    pub fn send(&mut self) -> IoResult<()> {
        self.write_status_line()?;
        self.write_headers()?;
        self.write_body()?;
        Ok(())    
    }

    /// Returns true if the Connection header is present with the value "close".
    #[must_use]
    pub fn has_close_connection_header(&self) -> bool {
        self.header(&CONNECTION).map_or(false,
            |val| {
                let val = String::from_utf8_lossy(val.as_bytes());
                "close".eq_ignore_ascii_case(val.trim())
            })
    }
}