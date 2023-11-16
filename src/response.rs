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
    pub body: Vec<u8>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            method: Method::default(),
            uri: String::from("/"),
            version: Version::default(),
            status: Status(200),
            headers: Self::default_headers(),
            body: Vec::new(),
        }
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.status_line())?;

        if !self.headers.is_empty() {
            for (name, value) in &self.headers {
                write!(f, "\n{name}: {value}")?;
            }
        }

        // Print the body if it is unencoded text or application MIME types.
        if !self.body.is_empty() && self.body_is_printable() {
            let body = String::from_utf8_lossy(&self.body);
            let body = body.trim();
            if !body.is_empty() {
                write!(f, "\n\n{body}")?;
            }
        }

        Ok(())
    }
}

impl Response {
    pub fn from_request(req: &Request, router: &Arc<Router>) -> NetResult<Self> {
        let resolved = router.resolve(req);

        let mut headers = BTreeMap::new();

        let body = {
            if let Some(path) = resolved.path() {
                let body = fs::read(path)?;
                let len = format!("{}", body.len());

                headers.insert(CACHE_CONTROL, HeaderValue::cache_control_from_path(path));
                headers.insert(CONTENT_LENGTH, len.as_str().into());
                headers.insert(CONTENT_TYPE, HeaderValue::content_type_from_path(path));

                body
            } else {
                Vec::new()
            }
        };

        let method = req.method;
        let uri = req.uri.clone();
        let version = req.version;
        let status = resolved.status;

        Ok(Self {
            method,
            uri,
            version,
            status,
            headers,
            body,
        })
    }

    #[must_use]
    pub const fn method(&self) -> &Method {
        &self.method
    }

    #[must_use]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    #[must_use]
    pub const fn version(&self) -> &Version {
        &self.version
    }

    #[must_use]
    pub const fn status(&self) -> &Status {
        &self.status
    }

    #[must_use]
    pub const fn status_code(&self) -> u16 {
        self.status.code()
    }

    #[must_use]
    pub const fn status_msg(&self) -> &'static str {
        self.status.msg()
    }

    #[must_use]
    pub const fn headers(&self) -> &HeadersMap {
        &self.headers
    }

    /// Default set of response headers.
    #[must_use]
    pub fn default_headers() -> HeadersMap {
        BTreeMap::from([
            (CACHE_CONTROL, "no-cache".into()),
            (CONTENT_LENGTH, "0".into()),
            (CONTENT_TYPE, "text/plain; charset=UTF-8".into()),
        ])
    }

    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains_key(name)
    }

    pub fn set_header(&mut self, name: HeaderName, val: HeaderValue) {
        if self.has_header(&name) {
            self.headers.entry(name).and_modify(|v| *v = val);
        } else {
            self.headers.insert(name, val);
        }
    }

    #[must_use]
    pub fn cache_control(&self) -> Option<&HeaderValue> {
        self.headers.get(&CACHE_CONTROL)
    }

    pub fn set_cache_control(&mut self, value: HeaderValue) {
        self.set_header(CACHE_CONTROL, value);
    }

    #[must_use]
    pub fn content_len(&self) -> Option<&HeaderValue> {
        self.headers.get(&CONTENT_LENGTH)
    }

    pub fn set_content_len(&mut self, value: HeaderValue) {
        self.set_header(CONTENT_LENGTH, value);
    }

    #[must_use]
    pub fn content_type(&self) -> Option<&HeaderValue> {
        self.headers.get(&CONTENT_TYPE)
    }

    pub fn set_content_type(&mut self, value: HeaderValue) {
        self.set_header(CONTENT_TYPE, value);
    }

    #[must_use]
    pub fn content_encoding(&self) -> Option<&HeaderValue> {
        self.headers.get(&CONTENT_ENCODING)
    }

    /// Returns true if the body is unencoded text or application MIME types.
    #[must_use]
    pub fn body_is_printable(&self) -> bool {
        if self.content_encoding().is_some() {
            return false;
        }

        self.content_type()
            .map_or(false, |value| match value.to_string() {
                ct if ct.contains("text") => true,
                ct if ct.contains("application") => true,
                _ => false,
            })
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        self.body.as_slice()
    }

    #[must_use]
    pub fn status_line(&self) -> String {
        format!("{} {}", &self.version, &self.status)
    }

    pub fn write_status_line(&self, writer: &mut RemoteConnect) -> IoResult<()> {
        write!(writer, "{} {}\r\n", &self.version, &self.status)
    }

    pub fn write_headers(&self, writer: &mut RemoteConnect) -> IoResult<()> {
        if !self.headers.is_empty() {
            self.headers.iter().for_each(|(name, value)| {
                write!(writer, "{name}: ").unwrap();
                writer.write_all(value.as_bytes()).unwrap();
                writer.write_all(b"\r\n").unwrap();
            });
        }

        // Signal the end of the headers section with an empty line.
        writer.write_all(b"\r\n")?;
        Ok(())
    }

    pub fn write_body(&self, writer: &mut RemoteConnect) -> IoResult<()> {
        if self.body.is_empty() {
            Ok(())
        } else {
            writer.write_all(&self.body)
        }
    }

    pub fn send(&self, writer: &mut RemoteConnect) -> IoResult<()> {
        self.write_status_line(writer)?;
        self.write_headers(writer)?;
        self.write_body(writer)?;
        writer.flush()?;
        Ok(())
    }
}
