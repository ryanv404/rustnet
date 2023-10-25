use std::{
    borrow::Cow,
    fmt, fs,
    io::{self, BufWriter, Write},
    net::TcpStream,
    path::Path,
};

use crate::{ArcRouter, Header, HeaderName, Request, Status, Version};

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

/// Content-Length header values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentLengthValue(pub u64);

impl fmt::Display for ContentLengthValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ContentLengthValue> for Header {
    fn from(value: ContentLengthValue) -> Self {
        Self {
            name: HeaderName::ContentLength,
            value: value.to_string().into_bytes()
        }
    }
}

impl From<u64> for ContentLengthValue {
    fn from(len: u64) -> Self {
        Self(len)
    }
}

impl From<usize> for ContentLengthValue {
    fn from(len: usize) -> Self {
        // usize should always fit in a u64 integer.
        let len = u64::try_from(len).unwrap();
        Self(len)
    }
}

/// Content-Type header values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentTypeValue {
    Application,
    Audio,
    Font,
    Example,
    Image,
    ImageXicon,
    Message,
    Model,
    Multipart,
    Text,
    TextHtml,
    TextPlain,
    Video,
}

impl fmt::Display for ContentTypeValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<ContentTypeValue> for Header {
    fn from(value: ContentTypeValue) -> Self {
        Self {
            name: HeaderName::ContentType,
            value: value.to_string().into_bytes()
        }
    }
}

impl ContentTypeValue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Audio => "audio",
            Self::Example => "example",
            Self::Font => "font",
            Self::Image => "image",
            Self::ImageXicon => "image/x-icon",
            Self::Message => "message",
            Self::Model => "model",
            Self::Multipart => "multipart",
            Self::Text => "text",
            Self::TextHtml => "text/html; charset=UTF-8",
            Self::TextPlain => "text/plain; charset=UTF-8",
            Self::Video => "video",
        }
    }

    #[must_use]
    pub fn from_path(path: &Path) -> &'static [u8] {
        if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("ico") => b"image/x-icon",
                Some("gif") => b"image/gif",
                Some("jpg") => b"image/jpeg",
                Some("jpeg") => b"image/jpeg",
                Some("png") => b"image/png",
                Some("pdf") => b"application/pdf",
                Some("html") | Some("htm") => b"text/html; charset=UTF-8",
                Some("txt") => b"text/plain; charset=UTF-8",
                _ => b"text/plain; charset=UTF-8",
            }
        } else {
            b"text/plain; charset=UTF-8"
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CacheControlValue {
    Immutable,
    MustRevalidate,
    MustUnderstand,
    NoCache,
    NoStore,
    NoTransform,
    Private,
    ProxyRevalidate,
    Public,
    StaleIfError,
    StaleWhileRevalidate,
    MaxAge(u64),
    SMaxAge(u64),
}

impl fmt::Display for CacheControlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<CacheControlValue> for Header {
    fn from(value: CacheControlValue) -> Self {
        Self {
            name: HeaderName::CacheControl,
            value: value.to_string().into_bytes()
        }
    }
}

impl CacheControlValue {
    pub fn as_str(&self) -> Cow<'static, str> {
        match self {
            Self::Immutable => "immutable".into(),
            Self::MustRevalidate => "must-revalidate".into(),
            Self::MustUnderstand => "must-understand".into(),
            Self::NoCache => "no-cache".into(),
            Self::NoStore => "no-store".into(),
            Self::NoTransform => "no-transform".into(),
            Self::Private => "private".into(),
            Self::ProxyRevalidate => "proxy-revalidate".into(),
            Self::Public => "public".into(),
            Self::StaleIfError => "stale-if-error".into(),
            Self::StaleWhileRevalidate => "stale-while-revalidate".into(),
            Self::MaxAge(age) => format!("max-age={age}").into(),
            Self::SMaxAge(age) => format!("s-maxage={age}").into(),
        }
    }

    #[must_use]
    pub fn from_path(path: &Path) -> Self {
        match path.extension() {
            // Allow caching of the favicon for 1 day.
            Some(ext) if ext == "ico" => CacheControlValue::MaxAge(86400),
            // Don't cache HTML pages during development.
            Some(_) | None => CacheControlValue::NoCache,
        }
    }
}

pub struct Response {
    version: Version,
    status: Status,
    headers: Vec<Header>,
    body: Vec<u8>,
}

impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let body = if self.body.is_empty() { "No content." } else { "..." };

        f.debug_struct("Response")
            .field("version", &self.version)
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body", &body)
            .finish()
    }
}

impl Response {
    #[must_use]
    pub fn from_request(req: &Request, router: &ArcRouter) -> io::Result<Self> {
        let version = req.version().clone();

        let (status, maybe_path) = {
            // Acquire lock in this block to minimize the time we hold it.
            let router_lock = router.lock().unwrap();
            router_lock.resolve(req)
        };

        let (cache_control, content_type, body) = match maybe_path {
            Some(path) => {
                let cache = Header::from(CacheControlValue::from_path(&path));
                let cont_type = Header::new(b"Content-Type", ContentTypeValue::from_path(&path));
                let data = fs::read(path)?;
                (cache, cont_type, data)
            },
            None => {
                let cache = Header::from(CacheControlValue::NoCache);
                let cont_type = Header::new(b"Content-Type", b"text/plain; charset=UTF-8");
                (cache, cont_type, Vec::new())
            },
        };

        let content_len = ContentLengthValue::from(body.len());
        let content_len = Header::from(content_len);
        let headers = vec![cache_control, content_len, content_type];

        Ok(Self { version, status, headers, body })
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
    pub fn has_header(&self, name: HeaderName) -> bool {
        self.headers.iter().any(|h| h.name == name)
    }

    #[must_use]
    pub fn get_header(&self, name: HeaderName) -> Option<&Header> {
        self.headers.iter().find(|&h| h.name == name)
    }

    #[must_use]
    pub fn cache_control(&self) -> Option<&Header> {
        self.get_header(HeaderName::CacheControl)
    }

    #[must_use]
    pub fn content_len(&self) -> Option<&Header> {
        self.get_header(HeaderName::ContentLength)
    }

    #[must_use]
    pub fn content_type(&self) -> Option<&Header> {
        self.get_header(HeaderName::ContentType)
    }

    #[must_use]
    pub fn headers(&self) -> &[Header] {
        &self.headers
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn status_line(&self) -> String {
        format!("{} {}\r\n", self.version(), self.status())
    }

    pub fn write_status_line(&self, writer: &mut BufWriter<TcpStream>) -> io::Result<()> {
        writer.write_all(&self.status_line().as_bytes())
    }

    pub fn write_headers(&self, writer: &mut BufWriter<TcpStream>) -> io::Result<()> {
        if !self.headers.is_empty() {
            for header in self.headers.iter() {
                writer.write_all(&header.to_bytes())?;
            }
        }
        // Signal the end of the headers section with an empty line.
        writer.write_all(b"\r\n")
    }

    pub fn write_body(&self, writer: &mut BufWriter<TcpStream>) -> io::Result<()> {
        if !self.body.is_empty() {
            writer.write_all(&self.body)?;
        }
        Ok(())
    }

    pub fn send(&self, writer: &mut BufWriter<TcpStream>) -> io::Result<()> {
        self.write_status_line(writer)?;
        self.write_headers(writer)?;
        self.write_body(writer)?;
        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_headers_search() {
        let cache = Header::from(CacheControlValue::NoCache);
        let c_type = Header::from(ContentTypeValue::TextHtml);
        let c_len = Header::from(ContentLengthValue::from(100u64));
        let headers = vec![cache.clone(), c_len.clone(), c_type.clone()];
        let res = Response {
            version: Version::OneDotOne, status: Status(200), headers, body: Vec::new()
        };
        assert_eq!(res.get_header(HeaderName::CacheControl), Some(&cache));
        assert_eq!(res.get_header(HeaderName::ContentLength), Some(&c_len));
        assert!(!res.has_header(HeaderName::Host));
        assert_ne!(res.get_header(HeaderName::ContentType), Some(&c_len));
    }
}
