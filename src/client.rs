use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufRead, Read, Write};
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use crate::consts::{
    CONTENT_LENGTH, CONTENT_TYPE, HOST, MAX_HEADERS,
};
use crate::{
    HeaderName, HeaderValue, HeadersMap, Method, NetError, NetReader,
    NetWriter, Request, Response, Status, Version,
};

/// Builder for the `Client` object.
#[derive(Clone, Debug)]
pub struct ClientBuilder<A: ToSocketAddrs> {
    pub method: Option<Method>,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub addr: Option<A>,
    pub uri: Option<String>,
    pub version: Option<Version>,
    pub headers: Option<HeadersMap>,
    pub body: Option<Vec<u8>>,
}

impl<A: ToSocketAddrs> Default for ClientBuilder<A> {
    fn default() -> Self {
        Self {
            method: None,
            ip: None,
            port: None,
            addr: None,
            uri: None,
            version: None,
            headers: None,
            body: None,
        }
    }
}

impl<A: ToSocketAddrs> ClientBuilder<A> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn method(&mut self, method: Method) -> &mut Self {
        self.method = Some(method);
        self
    }

    pub fn ip(&mut self, ip: &str) -> &mut Self {
        self.ip = Some(ip.to_string());
        self
    }

    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = Some(port);
        self
    }

    pub fn addr(&mut self, addr: A) -> &mut Self {
        self.addr = Some(addr);
        self
    }

    pub fn uri(&mut self, uri: &str) -> &mut Self {
        self.uri = Some(uri.to_string());
        self
    }

    pub fn version(&mut self, version: Version) -> &mut Self {
        self.version = Some(version);
        self
    }

    /// Returns the default request headers.
    #[must_use]
    pub fn default_headers(addr: &str) -> HeadersMap {
        Client::default_headers(addr)
    }

    /// Adds a header field to the request from a `HeaderName` and `HeaderValue`.
    pub fn header(&mut self, name: HeaderName, value: HeaderValue) -> &mut Self {
        if let Some(map) = self.headers.as_mut() {
            map.entry(name)
                .and_modify(|val| *val = value.clone())
                .or_insert(value);
        } else {
            self.headers = Some(BTreeMap::from([(name, value)]));
        }

        self
    }

    /// Adds a request body from a given bytes slice.
    pub fn body(&mut self, data: &[u8]) -> &mut Self {
        if !data.is_empty() {
            self.body = Some(data.to_vec());
        }
        self
    }

    /// Returns a `Client` instance from the builder.
    pub fn build(&mut self) -> IoResult<Client> {
        let stream = {
            if let (Some(ip), Some(port)) = (self.ip.as_ref(), self.port) {
                let remote = format!("{ip}:{port}");
                TcpStream::connect(remote)?
            } else if let Some(addr) = self.addr.as_ref() {
                TcpStream::connect(addr)?
            } else {
                return Err(IoError::from(IoErrorKind::InvalidInput));
            }
        };

        let (local_addr, remote_addr) = {
            let local = stream.local_addr()?;
            let remote = stream.peer_addr()?;
            (local, remote)
        };

        let (reader, writer) = {
            let (r, w) = (stream.try_clone()?, stream);
            (NetReader::from(r), NetWriter::from(w))
        };

        let method = self.method.take().unwrap_or_default();
        let uri = self.uri.take().unwrap_or_else(|| String::from("/"));
        let version = self.version.take().unwrap_or_default();

        let headers = self
            .headers
            .take()
            .unwrap_or_else(|| Self::default_headers(&remote_addr.to_string()));

        let body = self.body.take().unwrap_or_default();

        Ok(Client {
            method,
            uri,
            version,
            headers,
            body,
            local_addr,
            remote_addr,
            reader,
            writer,
        })
    }

    pub fn send(&mut self) -> IoResult<Client> {
        let mut client = self.build()?;
        client.send()?;
        Ok(client)
    }
}

/// An HTTP client that can send and receive messages with a remote host.
#[derive(Debug)]
pub struct Client {
    pub method: Method,
    pub uri: String,
    pub version: Version,
    pub headers: HeadersMap,
    pub body: Vec<u8>,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub reader: NetReader,
    pub writer: NetWriter,
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let request_line = self.request_line();
        let headers = self.headers_to_string();
        write!(f, "{request_line}\n{}", headers.trim())?;
        Ok(())
    }
}

impl Write for Client {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.writer.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.writer.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.writer.write_all(buf)
    }
}

impl Read for Client {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.reader.read(buf)
    }
}

impl BufRead for Client {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt);
    }
}

impl Client {
    /// Sends a GET request to the provided URL, returning a `Response`.
//    pub fn get(url: &str) -> IoResult<Response> {
//        let (addr, uri) = parse_url(url);
//
//        let stream = TcpStream::connect(addr)?;
//        let remote_addr = stream.peer_addr()?;
//
//        let req = Request {
//            uri,
//            remote_addr,
//            ..Request::default()
//        }
//
//        let res = Response::default();
//        Ok(res)
//    }

    /// Returns a new `ClientBuilder` instance.
    #[must_use]
    pub fn http<A: ToSocketAddrs>() -> ClientBuilder<A> {
        ClientBuilder::new()
    }

    /// Returns the request method.
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the requested URI.
    pub fn uri(&self) -> &str {
        &self.uri
    }

    /// Returns the request's protocol version.
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Default set of request headers.
    #[must_use]
    pub fn default_headers(host: &str) -> HeadersMap {
        let mut headers = Request::default_headers();
        headers.insert(HOST, host.into());
        headers
    }

    /// Returns a reference to the request headers map.
    pub const fn headers(&self) -> &HeadersMap {
        &self.headers
    }

    /// Returns all request headers as a String.
    pub fn headers_to_string(&self) -> String {
        if self.headers.is_empty() {
            String::new()
        } else {
            self.headers
                .iter()
                .fold(String::new(), |mut acc, (name, value)| {
                    let header = format!("{name}: {value}\r\n");
                    acc.push_str(&header);
                    acc
                })
        }
    }

    /// Returns the request's body as a slice.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Returns the client's socket address.
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns the remote server's socket address.
    pub const fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Returns the request line as a String.
    pub fn request_line(&self) -> String {
        format!("{} {} {}", self.method, self.uri, self.version)
    }

    /// Sends an HTTP request to the remote host.
    pub fn send(&mut self) -> IoResult<()> {
        // Request line
        let request_line = self.request_line();
        self.write_all(request_line.as_bytes())?;
        self.write_all(b"\r\n")?;

        // Request headers
        let headers = {
            let remote_addr = self.remote_addr().to_string();

            if self.headers.is_empty() {
                Self::default_headers(&remote_addr).iter().fold(
                    String::new(),
                    |mut acc, (name, value)| {
                        let header = format!("{name}: {value}\r\n");
                        acc.push_str(&header);
                        acc
                    },
                )
            } else {
                // Ensure the default headers are included.
                let default_headers = Request::default_headers();

                for (name, value) in default_headers {
                    self.headers.entry(name).or_insert(value);
                }

                self.headers_to_string()
            }
        };

        self.write_all(headers.as_bytes())?;
        self.write_all(b"\r\n")?;

        // Request body
        if !self.body.is_empty() {
            let len = self.body.len();

            // Ensure Content-Length and Content-Type are accurate.
            self.headers
                .entry(CONTENT_LENGTH)
                .and_modify(|val| *val = HeaderValue::from(len))
                .or_insert_with(|| HeaderValue::from(len));

            // Assume that the body is plain text if not previously set.
            self.headers
                .entry(CONTENT_TYPE)
                .or_insert_with(|| HeaderValue::from("plain/text; charset=UTF-8"));

            self.writer.write_all(&self.body)?;
        }

        self.flush()?;
        Ok(())
    }

    /// Receives an HTTP response from the remote host.
    pub fn recv(&mut self) -> IoResult<Response> {
        let uri = self.uri.clone();
        let method = self.method();

        let (version, status) = self.parse_status_line()?;

        let headers = self.parse_headers()?;
        let maybe_content_len = headers.get(&CONTENT_LENGTH);

        let body = self.parse_body(maybe_content_len).ok();

        Ok(Response {
            method,
            uri,
            version,
            status,
            headers,
            body,
        })
    }

    /// Parses the first line of a response into a `Version` and `Status`.
    pub fn parse_status_line(&mut self) -> IoResult<(Version, Status)> {
        let mut buf = String::new();

        match self.read_line(&mut buf) {
            Err(e) => Err(e),
            Ok(0) => Err(IoError::from(IoErrorKind::UnexpectedEof)),
            Ok(_) => {
                let line = buf.trim();

                if line.is_empty() {
                    let payload = "response status line is empty".to_string();
                    return Err(IoError::new(IoErrorKind::Other, payload));
                }

                let mut tok = line.splitn(3, ' ').map(str::trim);

                let tokens = (tok.next(), tok.next(), tok.next());

                let (Some(ver), Some(code), Some(msg)) = tokens else {
                    let payload = "cannot parse response status line".to_string();
                    return Err(IoError::new(IoErrorKind::Other, payload));
                };

                let Ok(version) = ver.parse::<Version>() else {
                    let payload = format!("cannot parse HTTP version: {ver}");
                    return Err(IoError::new(IoErrorKind::Other, payload));
                };

                if code.eq_ignore_ascii_case("200") {
                    Ok((version, Status(200)))
                } else if let Ok(status) = code.parse::<Status>() {
                    Ok((version, status))
                } else {
                    let payload = format!("cannot parse status code: {code} ({msg})");
                    Err(IoError::new(IoErrorKind::Other, payload))
                }
            }
        }
    }

    // Reads and parses the headers section into a BTreeMap.
    pub fn parse_headers(&mut self) -> IoResult<HeadersMap> {
        let mut num = 0;
        let mut line = String::new();
        let mut headers: HeadersMap = BTreeMap::new();

        while num <= MAX_HEADERS {
            line.clear();

            match self.read_line(&mut line) {
                Err(e) => return Err(e),
                Ok(0) => return Err(IoError::from(IoErrorKind::UnexpectedEof)),
                Ok(_) => {
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        return Ok(headers);
                    }

                    let (name, value) = Request::parse_header(trimmed)?;
                    headers.insert(name, value);
                    num += 1;
                }
            }
        }

        return Err(IoError::new(
            IoErrorKind::Other,
            String::from("too many headers")
        ));
    }

    pub fn parse_body(&mut self, content_len: Option<&HeaderValue>) -> IoResult<Vec<u8>> {
        match content_len {
            Some(value) => {
                let len_str = value.to_string();
                let len = len_str
                    .parse::<u32>()
                    .map_err(|_| NetError::ParseError("content-length"))?;

                if len == 0 {
                    return Ok(Vec::new());
                }

                let mut buf = Vec::with_capacity(len as usize);

                // Take by reference instead of consuming the reader.
                let mut reader_ref = self.reader.by_ref().take(u64::from(len));

                reader_ref.read_to_end(&mut buf)?;
                Ok(buf)
            }
            None => Ok(Vec::new()),
        }
    }
}
