use std::collections::BTreeMap;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufRead, Read, Write};
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use crate::{
    HeaderName, HeadersMap, HeaderValue, Method, NetError, NetReader, NetWriter,
    Response, Status, Version,
};
use crate::consts::{
    ACCEPT, HOST, USER_AGENT, UPGRADE_INSECURE_REQUESTS, CONTENT_LENGTH
};

#[derive(Clone, Debug)]
pub struct ClientBuilder<A: ToSocketAddrs> {
    method: Option<Method>,
    ip: Option<String>,
    port: Option<u16>,
    addr: Option<A>,
    uri: Option<String>,
    version: Option<Version>,
    headers: Option<HeadersMap>,
    body: Option<Vec<u8>>,
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

    pub fn default_headers(addr: &str) -> HeadersMap {
        Client::default_headers(addr)
    }

    pub fn set_header(&mut self, name: HeaderName, value: HeaderValue) -> &mut Self {
        if let Some(map) = self.headers.as_mut() {
            map.entry(name)
                .and_modify(|val| *val = value.clone())
                .or_insert(value);
        } else {
            self.headers = Some(BTreeMap::from([(name, value)]));
        }
        self
    }

    pub fn body(&mut self, content: &[u8]) -> &mut Self {
        if !content.is_empty() {
            self.body = Some(content.to_vec());
        }
        self
    }

    pub fn build(&mut self) -> IoResult<Client> {
        let stream = {
            if let (Some(ip), Some(port)) = (self.ip.as_ref(), self.port) {
                let remote = format!("{ip}:{port}");
                TcpStream::connect(&remote)?
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
        let uri = self.uri.take().unwrap_or(String::from("/"));
        let version = self.version.take().unwrap_or_default();

        let headers = self.headers.take().unwrap_or(
            Self::default_headers(&remote_addr.to_string())
        );

        let body = self.body.take().unwrap_or(Vec::new());

        Ok(Client {
            method,
            uri,
            version,
            headers,
            body,
            local_addr,
            remote_addr,
            reader,
            writer
        })
    }

    pub fn send(&mut self) -> IoResult<Client> {
        let mut client = self.build()?;
        client.send()?;
        Ok(client)
    }
}

#[derive(Debug)]
pub struct Client {
    method: Method,
    uri: String,
    version: Version,
    headers: HeadersMap,
    body: Vec<u8>,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    reader: NetReader,
    writer: NetWriter,
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let headers = self.headers_to_string();
        let request_line = self.get_request_line();
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
    pub fn http<A: ToSocketAddrs>() -> ClientBuilder<A> {
        ClientBuilder::new()
    }

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn version(&self) -> Version {
        self.version
    }

    /// Default set of request headers.
    #[must_use]
    pub fn default_headers(host: &str) -> HeadersMap {
        BTreeMap::from([
            (HOST, host.into()),
            (UPGRADE_INSECURE_REQUESTS, "0".into()),
            (ACCEPT, "text/html,application/json;q=0.9,*/*;q=0.8".into()),
            (USER_AGENT, "rustnet/0.1".into())
        ])
    }

    pub fn headers(&self) -> &HeadersMap {
        &self.headers
    }

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

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    pub fn get_request_line(&self) -> String {
        format!("{} {} {}", self.method, self.uri, self.version)
    }

    pub fn send(&mut self) -> IoResult<()> {
        // Request line
        let request_line = self.get_request_line();
        self.write_all(request_line.as_bytes())?;
        self.write_all(b"\r\n")?;

        // Headers
        let headers = {
            let remote_addr = self.remote_addr().to_string();

            if self.headers.is_empty() {
                Self::default_headers(&remote_addr)
                .iter()
                .fold(String::new(), |mut acc, (name, value)| {
                    let header = format!("{name}: {value}\r\n");
                    acc.push_str(&header);
                    acc
                })
            } else {
                // Ensure the default headers are included.
                let default_headers = Self::default_headers(&remote_addr);

                for (name, value) in default_headers.into_iter() {
                    if !self.headers.contains_key(&name) {
                        self.headers.insert(name, value);
                    }
                }

                self.headers_to_string()
            }
        };

        self.write_all(headers.as_bytes())?;
        self.write_all(b"\r\n")?;

        // Body
        if !self.body.is_empty() {
            self.writer.write_all(&self.body)?;
        }

        self.flush()?;
        Ok(())
    }

    pub fn recv(&mut self) -> IoResult<Response> {
        let mut buf = String::new();

        // Parse the response status line.
        let (version, status) = {
            match self.read_line(&mut buf) {
                Err(e) => return Err(e),
                Ok(0) => return Err(IoError::from(IoErrorKind::UnexpectedEof)),
                Ok(_) => Self::parse_status_line(&buf)?,
            }
        };

        let mut headers = BTreeMap::new();

        // Parse the response headers.
        loop {
            buf.clear();

            match self.read_line(&mut buf) {
                Err(e) => return Err(e),
                Ok(0) => return Err(IoError::from(IoErrorKind::UnexpectedEof)),
                Ok(_) => {
                    let trim = buf.trim();

                    // A blank line indicates the end of the headers section.
                    if trim.is_empty() {
                        break;
                    }

                    let (name, value) = Self::parse_header(trim)?;

                    headers.insert(name, value);
                }
            }
        }

        // Parse the request body.
        let body = match headers.get(&CONTENT_LENGTH) {
            Some(value) => {
                let len_str = value.to_string();
                let len = len_str.parse::<u32>()
                    .map_err(|_| NetError::ParseError("content length"))?;

                let mut body_buf = Vec::with_capacity(len as usize);

                let mut reader_ref = self
                    .reader
                    .by_ref()
                    .take(u64::from(len));

                reader_ref.read_to_end(&mut body_buf)?;

                body_buf
            },
            None => Vec::new(),
        };

        let method = self.method();
        let uri = self.uri.clone();

        Ok(Response {
            method,
            uri,
            version,
            status,
            headers,
            body
        })
    }

    /// Parses the first line of a response into a `Version` and `Status`.
    fn parse_status_line(line: &str) -> IoResult<(Version, Status)> {
        let trim = line.trim();

        if trim.is_empty() {
            return Err(IoError::from(IoErrorKind::InvalidData));
        }

        let mut tok = trim.splitn(3, ' ').map(str::trim);

        let tokens = (tok.next(), tok.next(), tok.next());

        let (Some(ver), Some(code), Some(msg)) = tokens else {
            return Err(IoError::from(IoErrorKind::InvalidData));
        };

        let ver = ver.parse::<Version>()?;

        if msg.eq_ignore_ascii_case("OK") {
            Ok((ver, Status(200)))
        } else if let Ok(status) = code.parse::<Status>() {
            Ok((ver, status))
        } else {
            Err(IoError::new(IoErrorKind::Other, format!("invalid status code: {code}")))
        }
    }

    /// Parses a string slice into a `HeaderName` and `HeaderValue`.
    pub fn parse_header(input: &str) -> IoResult<(HeaderName, HeaderValue)> {
        let mut tok = input.splitn(2, ':').map(str::trim);

        let tokens = (tok.next(), tok.next());

        if let (Some(name), Some(value)) = tokens {
            Ok((name.parse()?, value.into()))
        } else {
            Err(NetError::ParseError("request header").into())
        }
    }
}
