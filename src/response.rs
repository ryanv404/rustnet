use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::fs;
use std::io::{Result as IoResult, Write};
use std::sync::Arc;

use crate::consts::{CACHE_CONTROL, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE};
use crate::{
    HeaderName, HeaderValue, HeadersMap, Method, NetResult, RemoteConnect, Request, Router, Status,
    Version,
};

// A random HTTP response:
//HTTP/1.1 200 OK
//Accept-Ranges: bytes
//Age: 499402
//Cache-Control: max-age=604800
//Content-Encoding: gzip
//Content-Length: 648
//Content-Type: text/html; charset=UTF-8
//Date: Mon, 23 Oct 2023 20:14:46 GMT
//Etag: "3147526947+gzip"
//Expires: Mon, 30 Oct 2023 20:14:46 GMT
//Last-Modified: Thu, 17 Oct 2019 07:18:26 GMT
//Server: ECS (dcb/7EA3)
//Vary: Accept-Encoding
//X-Cache: HIT

#[derive(Debug)]
pub struct Response {
    pub method: Method,
    pub uri: String,
    pub version: Version,
    pub status: Status,
    pub headers: HeadersMap,
    pub body: Option<Vec<u8>>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            method: Method::default(),
            uri: String::from("/"),
            version: Version::default(),
            status: Status(200),
            headers: Self::default_headers(),
            body: None,
        }
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // The response status line.
        write!(f, "{}\r\n", self.status_line())?;

        // The response headers.
        if !self.headers.is_empty() {
            for (name, value) in &self.headers {
                write!(f, "{name}: {value}\r\n")?;
            }
        }

        // End of the headers section.
        write!(f, "\r\n")?;

        // Print the body if the current context allows for a message body and
        // if it is an unencoded text or application MIME type.
        if self.body.is_some() && self.body_is_permitted() && self.body_is_printable() {
            let body = self.body.as_ref().unwrap();
            let body = String::from_utf8_lossy(body);
            let body = body.trim();

            if !body.is_empty() {
                write!(f, "{body}")?;
            }
        }

        Ok(())
    }
}

impl Response {
    /// Parses a `Response` object from a `Request`.
    pub fn from_request(req: &Request, router: &Arc<Router>) -> NetResult<Self> {
        let resolved = router.resolve(req);
        let method = resolved.method;
        let status = resolved.status;

        let uri = req.uri.clone();
        let version = req.version;

        let mut headers = BTreeMap::new();

        let body = {
            if resolved.path.is_some() {
                let path = resolved.path.as_ref().unwrap();
                let content = fs::read(path)?;
                let len = content.len().to_string();

                headers.insert(CACHE_CONTROL, HeaderValue::cache_control_from_path(path));
                headers.insert(CONTENT_LENGTH, len.as_str().into());
                headers.insert(CONTENT_TYPE, HeaderValue::content_type_from_path(path));

                if method == Method::Head {
                    None
                } else {
                    Some(content)
                }
            } else {
                None
            }
        };

        Ok(Self {
            method,
            uri,
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

    /// Returns the target URI.
    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
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

    /// Returns the numeric status code.
    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status.code()
    }

    /// Returns the reason phrase for the status.
    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status.msg()
    }

    /// Returns a map of the response's headers.
    #[must_use]
    pub const fn headers(&self) -> &HeadersMap {
        &self.headers
    }

    /// A default set of response headers.
    #[must_use]
    pub fn default_headers() -> HeadersMap {
        BTreeMap::from([
            (CACHE_CONTROL, "no-cache".into()),
            (CONTENT_LENGTH, "0".into()),
            (CONTENT_TYPE, "text/plain; charset=UTF-8".into()),
        ])
    }

    /// Returns true if the header field represented by `HeaderName` is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains_key(name)
    }

    /// Adds or modifies the header field represented by the given `HeaderName`
    /// and `HeaderValue`.
    pub fn set_header(&mut self, name: HeaderName, val: HeaderValue) {
        if self.has_header(&name) {
            self.headers.entry(name).and_modify(|v| *v = val);
        } else {
            self.headers.insert(name, val);
        }
    }

    /// Returns the Cache-Control header value, if present.
    #[must_use]
    pub fn cache_control(&self) -> Option<&HeaderValue> {
        self.headers.get(&CACHE_CONTROL)
    }

    /// Adds or modifies the Cache-Control header field.
    pub fn set_cache_control(&mut self, value: HeaderValue) {
        self.set_header(CACHE_CONTROL, value);
    }

    /// Returns the Content-Length header value, if present.
    #[must_use]
    pub fn content_len(&self) -> Option<&HeaderValue> {
        self.headers.get(&CONTENT_LENGTH)
    }

    /// Adds or modifies the Content-Length header field.
    pub fn set_content_len(&mut self, value: HeaderValue) {
        self.set_header(CONTENT_LENGTH, value);
    }

    /// Returns the Content-Type header value, if present.
    #[must_use]
    pub fn content_type(&self) -> Option<&HeaderValue> {
        self.headers.get(&CONTENT_TYPE)
    }

    /// Adds or modifies the Content-Type header field.
    pub fn set_content_type(&mut self, value: HeaderValue) {
        self.set_header(CONTENT_TYPE, value);
    }

    /// Returns the Content-Encoding header value, if present.
    #[must_use]
    pub fn content_encoding(&self) -> Option<&HeaderValue> {
        self.headers.get(&CONTENT_ENCODING)
    }

    /// Returns true if the body is an unencoded text or application MIME type.
    #[must_use]
    pub fn body_is_printable(&self) -> bool {
        if self.content_encoding().is_some() || self.content_type().is_none() {
            return false;
        }

        let ctype = self.content_type().unwrap().to_string();

        ctype.contains("text") || ctype.contains("application")
    }

    /// Returns true if the `Response` is permitted to have a message body
    /// in the current context.
    #[must_use]
    pub fn body_is_permitted(&self) -> bool {
        match self.status_code() {
            // All responses with a 1xx (Informational), 204 (No Content), or
            // 304 (Not Modified) status lack a body.
            100..=199 | 204 | 304 => false,
            // All CONNECT responses with a 2xx (Success) status lack a body.
            200..=299 if self.method == Method::Connect => false,
            // All HEAD responses lack a body.
            _ if self.method == Method::Head => false,
            // Message bodies are permitted for all other responses.
            _ => true,
        }
    }

    /// Returns an optional reference to the message body, if present.
    #[must_use]
    pub const fn body(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }

    /// Returns a String representation of the response's status line.
    #[must_use]
    pub fn status_line(&self) -> String {
        format!("{} {}", &self.version, &self.status)
    }

    /// Writes the response's status line to the given stream.
    pub fn write_status_line(&self, writer: &mut RemoteConnect) -> IoResult<()> {
        write!(writer, "{} {}\r\n", &self.version, &self.status)?;
        Ok(())
    }

    /// Writes the response's headers to the given stream.
    pub fn write_headers(&self, writer: &mut RemoteConnect) -> IoResult<()> {
        if !self.headers.is_empty() {
            self.headers.iter().for_each(|(name, value)| {
                write!(writer, "{name}: ").unwrap();
                writer.write_all(value.as_bytes()).unwrap();
                writer.write_all(b"\r\n").unwrap();
            });
        }

        // Mark the end of the headers section.
        writer.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes the response's body to the given stream, if applicable.
    pub fn write_body(&self, writer: &mut RemoteConnect) -> IoResult<()> {
        if self.body.is_some() && self.body_is_permitted() {
            let body = self.body.as_ref().unwrap();
            writer.write_all(body)?;
        }

        Ok(())
    }

    /// Writes the `Response` to the underlying TCP connection that is
    /// established with the remote client.
    pub fn send(&self, writer: &mut RemoteConnect) -> IoResult<()> {
        self.write_status_line(writer)?;
        self.write_headers(writer)?;
        self.write_body(writer)?;

        writer.flush()?;
        Ok(())
    }
}
