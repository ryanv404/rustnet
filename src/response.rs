use std::{
    collections::HashMap,
    fs,
    io::{Result as IoResult, Write},
    sync::Arc,
};

use crate::{
    Header, HeaderName, Method, NetResult, RemoteClient, Request, RequestLine,
    Router, Status, Version, consts::{
        CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE
    },
};

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
    pub headers: HashMap<HeaderName, Header>,
    pub body: Vec<u8>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            method: Method::Get,
            uri: String::from("/"),
            version: Version::OneDotOne,
            status: Status(200),
            headers: Header::default_headers(),
            body: Vec::new(),
        }
    }
}

impl Response {
    pub fn from_request(req: &Request, router: &Arc<Router>) -> NetResult<Self> {
        let RequestLine {
            method,
            uri,
            version,
        } = req.request_line.clone();

        let resolved = router.resolve(req);

        let mut headers = HashMap::new();

        let body = {
            if let Some(path) = resolved.path() {
                let cache_con = Header::cache_control_from_path(path);
                let content_type = Header::content_type_from_path(path);

                headers.insert(cache_con.name.clone(), cache_con);
                headers.insert(content_type.name.clone(), content_type);

                fs::read(path)?
            } else {
                let cache_con = Header {
                    name: CACHE_CONTROL,
                    value: "no-cache".to_string(),
                };
                let content_type = Header {
                    name: CONTENT_TYPE,
                    value: "text/plain; charset=UTF-8".to_string(),
                };

                headers.insert(cache_con.name.clone(), cache_con);
                headers.insert(content_type.name.clone(), content_type);

                Vec::new()
            }
        };

        let content_len = Header {
            name: CONTENT_LENGTH,
            value: body.len().to_string(),
        };

        headers.insert(content_len.name.clone(), content_len);

        let status = resolved.status();

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
    pub const fn uri(&self) -> &String {
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
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains_key(name)
    }

    #[must_use]
    pub fn get_header(&self, name: &HeaderName) -> Option<&Header> {
        self.headers.get(name)
    }

    #[must_use]
    pub fn cache_control(&self) -> Option<&Header> {
        self.get_header(&CACHE_CONTROL)
    }

    pub fn set_cache_control(&mut self, value: &str) {
        self.headers.entry(CACHE_CONTROL)
            .and_modify(|hdr| { hdr.value = value.to_owned(); })
            .or_insert_with(|| Header {
                name: CACHE_CONTROL,
                value: value.to_owned(),
            });
    }

    #[must_use]
    pub fn content_len(&self) -> Option<&Header> {
        self.get_header(&CONTENT_LENGTH)
    }

    pub fn set_content_len(&mut self, len: u64) {
        self.headers.entry(CONTENT_LENGTH)
            .and_modify(|hdr| { hdr.value = len.to_string(); })
            .or_insert_with(|| Header {
                name: CONTENT_LENGTH,
                value: len.to_string(),
            });
    }

    #[must_use]
    pub fn content_type(&self) -> Option<&Header> {
        self.get_header(&CONTENT_TYPE)
    }

    pub fn set_content_type(&mut self, content_type: &str) {
        self.headers.entry(CONTENT_TYPE)
            .and_modify(|hdr| { hdr.value = content_type.to_owned(); })
            .or_insert_with(|| Header {
                name: CONTENT_TYPE,
                value: content_type.to_owned(),
            });
    }

    /// Returns a vector of all headers sorted by header name.
    #[must_use]
    pub fn all_headers(&self) -> Vec<Header> {
        let mut headers: Vec<Header> = self.headers.clone().into_values().collect();
        headers.sort_unstable();
        headers
    }

    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    #[must_use]
    pub fn status_line(&self) -> String {
        format!("{} {}", self.version(), self.status())
    }

    pub fn write_status_line(&self, writer: &mut RemoteClient) -> IoResult<()> {
        write!(writer, "{} {}\r\n", self.version(), self.status())
    }

    pub fn write_headers(&self, writer: &mut RemoteClient) -> IoResult<()> {
        if !self.headers.is_empty() {
            self.all_headers().iter().for_each(|h| {
                write!(writer, "{h}\r\n").unwrap();
            });
        }

        // Signal the end of the headers section with an empty line.
        writer.write_all(b"\r\n")?;
        Ok(())
    }

    pub fn write_body(&self, writer: &mut RemoteClient) -> IoResult<()> {
        if !self.body.is_empty() {
            writer.write_all(&self.body)?;
        }

        Ok(())
    }

    pub fn send(&self, writer: &mut RemoteClient) -> IoResult<()> {
        self.write_status_line(writer)?;
        self.write_headers(writer)?;
        self.write_body(writer)?;

        writer.flush()?;
        Ok(())
    }
}
