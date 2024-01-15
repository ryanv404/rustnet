use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{
    BufRead, BufReader, BufWriter, Read, Result as IoResult, Write,
};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::process;
use std::str;

use crate::{
    Body, Headers, Method, NetError, NetResult, Request, Response, Status,
    UriPath, Version, MAX_HEADERS, READER_BUFSIZE, WRITER_BUFSIZE,
};
use crate::headers::names::{CONNECTION, CONTENT_LENGTH, CONTENT_TYPE};
use crate::style::colors::{RED, RESET};

/// A trait for printing CLI argument errors to the terminal.
pub trait WriteCliError {
    /// Prints unknown option error message and exits the program.
    fn unknown_opt(&self, name: &str) {
        eprintln!("{RED}Unknown option: `{name}`{RESET}");
        process::exit(1);
    }

    /// Prints unknown argument error message and exits the program.
    fn unknown_arg(&self, name: &str) {
        eprintln!("{RED}Unknown argument: `{name}`{RESET}");
        process::exit(1);
    }

    /// Prints missing argument error message and exits the program.
    fn missing_arg(&self, name: &str) {
        eprintln!("{RED}Missing `{name}` argument.{RESET}");
        process::exit(1);
    }

    /// Prints invalid argument error message and exits the program.
    fn invalid_arg(&self, name: &str, arg: &str) {
        eprintln!("{RED}Invalid `{name}` argument: \"{arg}\"{RESET}");
        process::exit(1);
    }
}

/// Represents the TCP connection between a client and a server.
#[derive(Debug)]
pub struct Connection {
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub reader: BufReader<TcpStream>,
    pub writer: BufWriter<TcpStream>,
}

impl Display for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "Connection {{")?;
        writeln!(f, "    local_addr: {},", self.local_addr)?;
        writeln!(f, "    remote_addr: {},", self.remote_addr)?;
        writeln!(f, "    reader: BufReader {{ TcpStream {{ ... }} }},")?;
        writeln!(f, "    writer: BufWriter {{ TcpStream {{ ... }} }},")?;
        write!(f, "}}")?;
        Ok(())
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

impl TryFrom<&str> for Connection {
    type Error = NetError;

    fn try_from(addr: &str) -> NetResult<Self> {
        TcpStream::connect(addr)
            .map_err(|e| NetError::IoError(e.kind()))
            .and_then(Self::try_from)
    }
}

impl TryFrom<TcpStream> for Connection {
    type Error = NetError;

    fn try_from(stream: TcpStream) -> NetResult<Self> {
        let remote_addr = stream.peer_addr()?;
        Self::try_from((stream, remote_addr))
    }
}

impl TryFrom<(TcpStream, SocketAddr)> for Connection {
    type Error = NetError;

    fn try_from(
        (stream, remote_addr): (TcpStream, SocketAddr)
    ) -> NetResult<Self> {
        let local_addr = stream.local_addr()?;

        let clone = stream.try_clone()?;
        let reader = BufReader::with_capacity(READER_BUFSIZE, clone);
        let writer = BufWriter::with_capacity(WRITER_BUFSIZE, stream);

        Ok(Self { local_addr, remote_addr, reader, writer })
    }
}

impl Connection {
    /// Returns the IP address for the remote half of the `TcpStream`.
    #[must_use]
    pub const fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
    }

    /// Returns the port for the remote half of the `TcpStream`.
    #[must_use]
    pub const fn remote_port(&self) -> u16 {
        self.remote_addr.port()
    }

    /// Returns the IP address for the local half of the `TcpStream`.
    #[must_use]
    pub const fn local_ip(&self) -> IpAddr {
        self.local_addr.ip()
    }

    /// Returns the port for the local half of the `TcpStream`.
    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.local_addr.port()
    }

    /// Returns a clone of this `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if cloning of the contained `TcpStream` fails.
    pub fn try_clone(&self) -> NetResult<Self> {
        let local_addr = self.local_addr;
        let remote_addr = self.remote_addr;

        let reader = self
            .reader
            .get_ref()
            .try_clone()
            .map(|stream| BufReader::with_capacity(READER_BUFSIZE, stream))?;

        let writer = self
            .writer
            .get_ref()
            .try_clone()
            .map(|stream| BufWriter::with_capacity(WRITER_BUFSIZE, stream))?;

        Ok(Self { local_addr, remote_addr, reader, writer })
    }

    /// Reads a single line from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read from the underlying `TcpStream` returns `Ok(0)`.
    pub fn recv_line(&mut self, buf: &mut Vec<u8>) -> NetResult<()> {
        let max_bytes = u64::try_from(READER_BUFSIZE).unwrap_or(4000);
        let mut reader = self.reader.by_ref().take(max_bytes);

        match reader.read_until(b'\n', buf) {
            Err(e) => Err(NetError::Read(e.kind())),
            Ok(0) => Err(NetError::UnexpectedEof),
            Ok(_) => Ok(()),
        }
    }

    /// Reads and parses all headers from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// As with the other readers, an error of kind `NetError::UnexpectedEof`
    /// is returned if `Ok(0)` is received while reading from the underlying
    /// `TcpStream`.
    pub fn recv_headers(&mut self, buf: &mut Vec<u8>) -> NetResult<Headers> {
        let max_bytes = u64::try_from(READER_BUFSIZE).unwrap_or(4000);
        let mut reader = self.reader.by_ref().take(max_bytes);

        let mut num_headers = 0;

        loop {
            if num_headers >= MAX_HEADERS {
                return Err(NetError::TooManyHeaders);
            }

            match reader.read_until(b'\n', buf) {
                Err(e) => return Err(NetError::Read(e.kind())),
                Ok(0) => return Err(NetError::UnexpectedEof),
                Ok(1 | 2) => return Headers::try_from(buf.as_slice()),
                Ok(_) => num_headers += 1,
            }
        }
    }

    /// Reads and parses the message body from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`.
    pub fn recv_body(
        &mut self,
        buf: &mut Vec<u8>,
        headers: &Headers
    ) -> NetResult<Body> {
        let content_len = headers
            .get(&CONTENT_LENGTH)
            .and_then(|value| value.as_str().parse::<u64>().ok())
            .unwrap_or(0);

        let content_type = headers
            .get(&CONTENT_TYPE)
            .map_or(Cow::Borrowed(""), |value| value.as_str());

        if content_len == 0 {
            Ok(Body::Empty)
        } else {
            self.reader.by_ref().take(content_len).read_to_end(buf)?;
            Ok(Body::from_content_type(buf, &content_type))
        }
    }

    /// Reads and parses a `Request` from a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to read or parse the
    /// individual components of the `Request`.
    pub fn recv_request(&mut self) -> NetResult<Request> {
        let mut buf = Vec::with_capacity(READER_BUFSIZE);

        self.recv_line(&mut buf)?;
        let (method, path, version) = Request::parse_request_line(&buf)?;

        buf.clear();

        let headers = self.recv_headers(&mut buf)?;

        buf.clear();

        let body = self.recv_body(&mut buf, &headers)?;

        Ok(Request {method, path, version, headers, body })
    }

    /// Reads and parses a `Response` from a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to read or parse the
    /// individual components of the `Response`.
    pub fn recv_response(&mut self) -> NetResult<Response> {
        let mut buf = Vec::with_capacity(READER_BUFSIZE);

        self.recv_line(&mut buf)?;
        let (version, status) = Response::parse_status_line(&buf)?;

        buf.clear();

        let headers = self.recv_headers(&mut buf)?;

        buf.clear();

        let body = self.recv_body(&mut buf, &headers)?;

        Ok(Response { version, status, headers, body })
    }

    /// Writes the request line to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the request line could not be written
    /// to the underlying `TcpStream` successfully.
    pub fn write_request_line(
        &mut self,
        method: &Method,
        path: &UriPath,
        version: &Version
    ) -> NetResult<()> {
        self.writer.write_all(method.as_bytes())?;
        self.writer.write_all(b" ")?;
        self.writer.write_all(path.as_bytes())?;
        self.writer.write_all(b" ")?;
        self.writer.write_all(version.as_bytes())?;
        self.writer.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes the status line to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the status line could not be written
    /// to the underlying `TcpStream` successfully.
    pub fn write_status_line(
        &mut self,
        version: &Version,
        status: &Status
    ) -> NetResult<()> {
        self.writer.write_all(version.as_bytes())?;
        self.writer.write_all(b" ")?;
        self.writer.write_all(&status.as_bytes())?;
        self.writer.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes a `Headers` map to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if a problem was encountered while writing the
    /// `Headers` to the underlying `TcpStream`.
    pub fn write_headers(&mut self, headers: &Headers) -> NetResult<()> {
        for (name, value) in &headers.0 {
            self.writer.write_all(name.as_bytes())?;
            self.writer.write_all(b": ")?;
            self.writer.write_all(value.as_bytes())?;
            self.writer.write_all(b"\r\n")?;
        }

        self.writer.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes a message `Body` to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the `Body` could not be written
    /// to the underlying `TcpStream` successfully.
    pub fn write_body(&mut self, body: &Body) -> NetResult<()> {
        if !body.is_empty() {
            self.writer.write_all(body.as_bytes())?;
        }

        Ok(())
    }

    /// Writes a `Request` to a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to write any of the
    /// individual components of the `Request` to the `TcpStream`.
    pub fn send_request(&mut self, req: &mut Request) -> NetResult<()> {
        // Ensure default request headers are set.
        req.headers.default_request_headers(&req.body, Some(self.remote_addr));

        self.write_request_line(&req.method, &req.path, &req.version)?;
        self.write_headers(&req.headers)?;
        self.write_body(&req.body)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Writes a `Response` to a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to write any of the
    /// individual components of the `Response` to the `TcpStream`.
    pub fn send_response(&mut self, res: &mut Response) -> NetResult<()> {
        // Ensure default response headers are set.
        res.headers.default_response_headers(&res.body);

        self.write_status_line(&res.version, &res.status)?;
        self.write_headers(&res.headers)?;
        self.write_body(&res.body)?;
        self.writer.flush()?;
        Ok(())
    }

    /// Writes an error `Response` to the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if writing to the underlying `TcpStream` fails
    /// or if the provided status `code` is not in the range from 100 to 999,
    /// inclusive.
    pub fn send_error(&mut self, code: u16, msg: String) -> NetResult<()> {
        let body = Body::from(msg);
        let version = Version::default();
        let status = Status::try_from(code)?;

        let mut headers = Headers::new();
        headers.default_response_headers(&body);
        headers.insert(CONNECTION, "close".into());

        self.write_status_line(&version, &status)?;
        self.write_headers(&headers)?;
        self.write_body(&body)?;
        self.writer.flush()?;
        Ok(())
    }
}
