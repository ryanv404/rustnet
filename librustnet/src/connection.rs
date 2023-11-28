use std::io::{
    BufRead, BufReader, BufWriter, ErrorKind as IoErrorKind,
    Read, Result as IoResult, Write,
};
use std::net::{IpAddr, SocketAddr, TcpStream};

use crate::consts::{
    ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, HOST, MAX_HEADERS,
    READER_BUFSIZE, SERVER, USER_AGENT, WRITER_BUFSIZE,
};
use crate::{
    Body, Header, Headers, NetError, NetResult, ParseErrorKind,
    Request, RequestLine, Response, StatusLine,
};

#[derive(Debug)]
pub struct NetReader(pub BufReader<TcpStream>);

impl From<TcpStream> for NetReader {
    fn from(stream: TcpStream) -> Self {
        Self(BufReader::with_capacity(READER_BUFSIZE, stream))
    }
}

impl Read for NetReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.0.read(buf)
    }
}

impl BufRead for NetReader {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.0.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.0.consume(amt);
    }
}

impl NetReader {
    /// Returns a clone of the current `NetReader` instance.
    pub fn try_clone(&self) -> NetResult<Self> {
        let stream = self.0.get_ref().try_clone()?;
        Ok(Self::from(stream))
    }

    /// Reads an HTTP request from the underlying `TcpStream`.
    pub fn recv_request(&mut self) -> NetResult<Request> {
        let request_line = self.read_request_line()?;
        let headers = self.read_headers()?;
        let body = self.read_body(&headers)?;
        let reader = Some(self.try_clone()?);

        Ok(Request { request_line, headers, body, reader })
    }

    /// Reads an HTTP response from the underlying `TcpStream`.
    pub fn recv_response(&mut self) -> NetResult<Response> {
        let status_line = self.read_status_line()?;
        let headers = self.read_headers()?;
        let body = self.read_body(&headers)?;
        let writer = Some(NetWriter::try_from(&*self)?);

        Ok(Response { status_line, headers, body, writer })
    }

    /// Reads a request line from the underlying `TcpStream`.
    pub fn read_request_line(&mut self) -> NetResult<RequestLine> {
        let mut line = String::with_capacity(1024);

        match self.read_line(&mut line) {
            Err(e) => Err(NetError::ReadError(e.kind())),
            Ok(0) => Err(IoErrorKind::UnexpectedEof.into()),
            Ok(_) => RequestLine::parse(&line),
        }
    }

    /// Reads a response status line from the underlying `TcpStream`.
    pub fn read_status_line(&mut self) -> NetResult<StatusLine> {
        let mut line = String::with_capacity(1024);

        match self.read_line(&mut line) {
            Err(e) => Err(NetError::ReadError(e.kind())),
            Ok(0) => Err(IoErrorKind::UnexpectedEof.into()),
            Ok(_) => StatusLine::parse(&line),
        }
    }

    /// Reads request headers from the underlying `TcpStream`.
    pub fn read_headers(&mut self) -> NetResult<Headers> {
        let mut num_headers = 0;
        let mut headers = Headers::new();
        let mut buf = String::with_capacity(1024);

        while num_headers <= MAX_HEADERS {
            match self.read_line(&mut buf) {
                Err(e) => return Err(NetError::ReadError(e.kind())),
                Ok(0) => return Err(IoErrorKind::UnexpectedEof)?,
                Ok(_) => {
                    let line = buf.trim();

                    if line.is_empty() {
                        break;
                    }

                    let (name, value) = Header::parse(line)?;
                    headers.insert(name, value);

                    buf.clear();
                    num_headers += 1;
                }
            }
        }

        Ok(headers)
    }

    /// Reads and parses the message body based on the value of the
    /// Content-Length and Content-Type headers.
    pub fn read_body(&mut self, headers: &Headers) -> NetResult<Body> {
        let content_len = headers.get(&CONTENT_LENGTH);
        let content_type = headers.get(&CONTENT_TYPE);

        if content_len.is_none() || content_type.is_none() {
            return Ok(Body::Empty);
        }

        let body_len = content_len
            .ok_or_else(|| NetError::ParseError(ParseErrorKind::Body))
            .map(|hdr_val| hdr_val.to_string())
            .and_then(|s| s.trim().parse::<usize>()
                .map_err(|_| NetError::ParseError(ParseErrorKind::Body)))?;

        if body_len == 0 {
            return Ok(Body::Empty);
        }

        let num_bytes = u64::try_from(body_len)
            .map_err(|_| NetError::ParseError(ParseErrorKind::Body))?;

        let body_type = content_type
            .ok_or_else(|| NetError::ParseError(ParseErrorKind::Body))
            .map(|hdr_val| hdr_val.to_string())?;

        if body_type.is_empty() {
            // Return error since content length is greater than zero.
            return Err(NetError::ParseError(ParseErrorKind::Body));
        }

        let mut reader = self.take(num_bytes);
        let mut buf = Vec::with_capacity(body_len);

        // TODO: handle chunked data and partial reads.
        reader.read_to_end(&mut buf)?;

        let mut type_tokens = body_type.splitn(2, '/');

        match type_tokens.next().map(|s| s.trim()) {
            Some("text") => match type_tokens.next().map(|s| s.trim()) {
                Some(s) if s.starts_with("html") => {
                    Ok(Body::Text(String::from_utf8_lossy(&buf).to_string()))
                },
                Some(s) if s.starts_with("plain") => {
                    Ok(Body::Text(String::from_utf8_lossy(&buf).to_string()))
                },
                _ => Ok(Body::Text(String::from_utf8_lossy(&buf).to_string())),
            },
            Some("application") => match type_tokens.next().map(|s| s.trim()) {
                Some(s) if s.starts_with("json") => Ok(Body::Json(String::from_utf8_lossy(&buf).to_string())),
                Some(s) if s.starts_with("xml") => Ok(Body::Xml(String::from_utf8_lossy(&buf).to_string())),
                Some(s) if s.starts_with("octet-stream") => Ok(Body::Bytes(buf)),
                _ => Ok(Body::Bytes(buf)),
            },
            Some("image") => match type_tokens.next().map(|s| s.trim()) {
                Some(s) if s.starts_with("x-icon") => Ok(Body::Favicon(buf)),
                Some(s) if s.starts_with("png") => Ok(Body::Image(buf)),
                Some(s) if s.starts_with("jpeg") => Ok(Body::Image(buf)),
                Some(s) if s.starts_with("gif") => Ok(Body::Image(buf)),
                _ => Ok(Body::Image(buf)),
            },
            _ => Ok(Body::Bytes(buf)),
        }
    }
}

#[derive(Debug)]
pub struct NetWriter(pub BufWriter<TcpStream>);

impl From<TcpStream> for NetWriter {
    fn from(stream: TcpStream) -> Self {
        Self(BufWriter::with_capacity(WRITER_BUFSIZE, stream))
    }
}

impl TryFrom<&NetReader> for NetWriter {
    type Error = NetError;

    fn try_from(reader: &NetReader) -> NetResult<Self> {
        let stream = reader.0.get_ref().try_clone()?;
        Ok(Self::from(stream))
    }
}

impl Write for NetWriter {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.0.flush()
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.0.write_all(buf)
    }
}

impl NetWriter {
    /// Returns a clone of the current `NetWriter` instance.
    pub fn try_clone(&self) -> NetResult<Self> {
        let stream = self.0.get_ref().try_clone()?;
        Ok(Self::from(stream))
    }

    /// Writes an HTTP request to the underlying `TcpStream`.
    pub fn send_request(&mut self, req: &mut Request) -> NetResult<()> {
        if !req.headers.contains(&ACCEPT) {
            req.headers.insert_accept("*/*");
        }

        if !req.headers.contains(&HOST) {
            let stream = self.0.get_ref();
            let remote = stream.peer_addr()?;
            req.headers.insert_host(remote.ip(), remote.port());
        }

        if !req.headers.contains(&USER_AGENT) {
            req.headers.insert_user_agent();
        }

        self.write_all(format!("{}\r\n", &req.request_line).as_bytes())?;
        self.write_headers(&req.headers)?;
        self.write_body(&req.body)?;

        self.flush()?;
        Ok(())
    }

    /// Writes an HTTP response to the underlying `TcpStream`.
    pub fn send_response(&mut self, res: &mut Response) -> NetResult<()> {
        if !res.headers.contains(&SERVER) {
            res.headers.insert_server();
        }

        self.write_all(format!("{}\r\n", &res.status_line).as_bytes())?;
        self.write_headers(&res.headers)?;
        self.write_body(&res.body)?;

        self.flush()?;
        Ok(())
    }

    /// Writes the response headers to the underlying `TcpStream`.
    pub fn write_headers(&mut self, headers: &Headers) -> NetResult<()> {
        if !headers.is_empty() {
            for (name, value) in headers.0.iter() {
                self.write_all(format!("{name}: {value}\r\n").as_bytes())?;
            }
        }

        self.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes the response body to the underlying `TcpStream`.
    pub fn write_body(&mut self, body: &Body) -> NetResult<()> {
        if !body.is_empty() {
            self.write_all(body.as_bytes())?;
        }

        Ok(())
    }
}

/// Represents a TCP connection to a remote client.
#[derive(Debug)]
pub struct Connection {
    /// The local socket address.
    pub local_addr: SocketAddr,
    /// The remote socket address.
    pub remote_addr: SocketAddr,
    /// Reads requests from the TCP connection.
    pub reader: NetReader,
    /// Writes responses to the TCP connection.
    pub writer: NetWriter,
}

impl TryFrom<(TcpStream, SocketAddr)> for Connection {
    type Error = NetError;

    fn try_from((stream, addr): (TcpStream, SocketAddr)) -> NetResult<Self> {
        let conn = Self::new(stream, Some(addr))?;
        Ok(conn)
    }
}

impl TryFrom<TcpStream> for Connection {
    type Error = NetError;

    fn try_from(stream: TcpStream) -> NetResult<Self> {
        let conn = Self::new(stream, None)?;
        Ok(conn)
    }
}

impl TryFrom<NetReader> for Connection {
    type Error = NetError;

    fn try_from(reader: NetReader) -> NetResult<Self> {
        let stream = reader.0.get_ref().try_clone()?;
        let conn = Self::new(stream, None)?;
        Ok(conn)
    }
}

impl Write for Connection {
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

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.reader.read(buf)
    }
}

impl BufRead for Connection {
    fn fill_buf(&mut self) -> IoResult<&[u8]> {
        self.reader.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.reader.consume(amt);
    }
}

impl Connection {
    /// Creates a new readable and writable `Connection` instance.
    pub fn new(stream: TcpStream, maybe_addr: Option<SocketAddr>) -> NetResult<Self> {
        let (remote_addr, local_addr) = match maybe_addr {
            Some(addr) => (addr, stream.local_addr()?),
            None => (stream.peer_addr()?, stream.local_addr()?),
        };

        let reader = NetReader::from(stream.try_clone()?);
        let writer = NetWriter::from(stream);

        Ok(Self { local_addr, remote_addr, reader, writer })
    }

    /// Returns the local client's socket address.
    #[must_use]
    pub const fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns the local client's IP address.
    #[must_use]
    pub const fn local_ip(&self) -> IpAddr {
        self.local_addr.ip()
    }

    /// Returns the local client's port.
    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.local_addr.port()
    }

    /// Returns the local client's socket address.
    #[must_use]
    pub const fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    /// Returns the remote host's IP address.
    #[must_use]
    pub const fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
    }

    /// Returns the remote host's port.
    #[must_use]
    pub const fn remote_port(&self) -> u16 {
        self.remote_addr.port()
    }

    /// Attempts to clones this `Connection` object.
    pub fn try_clone(&self) -> NetResult<Self> {
        let stream = self.reader.0.get_ref().try_clone()?;
        let conn = Self::try_from(stream)?;
        Ok(conn)
    }
}
