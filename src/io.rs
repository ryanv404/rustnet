use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{
    BufRead, BufReader, BufWriter, Read, Result as IoResult, Write,
};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::process;
use std::str;

use crate::{
    Body, Headers, Method, NetError, NetParseError, NetResult, Request,
    Response, Status, UriPath, Version, DEFAULT_NAME,
};
use crate::header::MAX_HEADERS;
use crate::header::names::{CONTENT_LENGTH, CONTENT_TYPE, HOST, SERVER};
use crate::style::colors::{BR_RED, CLR};

pub const READER_BUFSIZE: usize = 2048;
pub const WRITER_BUFSIZE: usize = 2048;

/// Represents the TCP connection between a client and a server.
#[derive(Debug)]
pub struct Connection {
    pub reader: BufReader<TcpStream>,
    pub writer: BufWriter<TcpStream>,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
}

impl Display for Connection {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "Connection {{")?;
        writeln!(f, "    reader: BufReader {{ TcpStream {{ ... }} }},")?;
        writeln!(f, "    writer: BufWriter {{ TcpStream {{ ... }} }},")?;
        writeln!(f, "    local_addr: {},", self.local_addr)?;
        writeln!(f, "    remote_addr: {},", self.remote_addr)?;
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
            .map_err(|_| NetError::ConnectFailure)
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

        Ok(Self { reader, writer, local_addr, remote_addr })
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

        Ok(Self { reader, writer, local_addr, remote_addr })
    }

    /// Reads and parses the request line from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`.
    pub fn recv_request_line(
        &mut self
    ) -> NetResult<(Method, UriPath, Version)> {
        let reader_ref = Read::by_ref(self);
        let mut reader = reader_ref.take(2024);

        let mut buf = String::with_capacity(2024);

        let request_line = match reader.read_line(&mut buf) {
            Ok(0) => Err(NetError::UnexpectedEof)?,
            Ok(_) => Request::parse_request_line(&buf)?,
            Err(e) => Err(NetError::Read(e.kind()))?,
        };

        Ok(request_line)
    }

    /// Reads and parses the status line from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`.
    pub fn recv_status_line(
        &mut self
    ) -> NetResult<(Version, Status)> {
        let reader_ref = Read::by_ref(self);
        let mut reader = reader_ref.take(2024);

        let mut buf = String::with_capacity(2024);

        let status_line = match reader.read_line(&mut buf) {
            Ok(0) => Err(NetError::UnexpectedEof)?,
            Ok(_) => Response::parse_status_line(&buf)?,
            Err(e) => Err(NetError::Read(e.kind()))?,
        };

        Ok(status_line)
    }

    /// Reads and parses all headers from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// As with the other readers, an error of kind `NetError::UnexpectedEof`
    /// is returned if `Ok(0)` is received while reading from the underlying
    /// `TcpStream`.
    pub fn recv_headers(&mut self, buf: &mut Vec<u8>) -> NetResult<Headers> {
        let reader_ref = Read::by_ref(self);
        let mut reader = reader_ref.take(2024);

        let mut num_headers = 0;

        while num_headers < MAX_HEADERS {
            match reader.read_until(b'\n', buf) {
                Ok(0) => Err(NetError::UnexpectedEof)?,
                Ok(1 | 2) => return Headers::try_from(buf),
                Ok(_) => num_headers += 1,
                Err(e) => Err(NetError::Read(e.kind()))?,
            }
        }

        Err(NetParseError::TooManyHeaders)?
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
        content_len: u64,
        content_type: &str
    ) -> NetResult<Body> {
        let reader_ref = Read::by_ref(self);
        let mut reader = reader_ref.take(content_len);

        reader.read_to_end(buf)?;

        Ok(Body::from_content_type(buf, content_type))
    }

    /// Reads and parses a `Request` from a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to read or parse the
    /// individual components of the `Request`.
    pub fn recv_request(&mut self) -> NetResult<Request> {
        let (method, path, version) = self.recv_request_line()?;

        let mut buf = Vec::with_capacity(1024);

        let headers = self.recv_headers(&mut buf)?;

        let content_len = headers
            .get(&CONTENT_LENGTH)
            .and_then(|value| value.as_str().trim().parse::<u64>().ok())
            .unwrap_or(0);

        let content_type = headers
            .get(&CONTENT_TYPE)
            .map(|value| value.as_str())
            .unwrap_or(Cow::Borrowed(""));

        buf.clear();

        let body = if content_len == 0 {
            Body::Empty
        } else {
            self.recv_body(
                &mut buf,
                content_len,
                &content_type
            )?
        };

        Ok(Request {method, path, version, headers, body })
    }

    /// Reads and parses a `Response` from a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to read or parse the
    /// individual components of the `Response`.
    pub fn recv_response(&mut self) -> NetResult<Response> {
        let (version, status) = self.recv_status_line()?;

        let mut buf = Vec::with_capacity(1024);

        let headers = self.recv_headers(&mut buf)?;

        let content_len = headers
            .get(&CONTENT_LENGTH)
            .and_then(|value| value.as_str().trim().parse::<u64>().ok())
            .unwrap_or(0);

        let content_type = headers
            .get(&CONTENT_TYPE)
            .map(|value| value.as_str())
            .unwrap_or(Cow::Borrowed(""));

        buf.clear();

        let body = if content_len == 0 {
            Body::Empty
        } else {
            self.recv_body(
                &mut buf,
                content_len,
                &content_type
            )?
        };

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
        self.write_all(method.as_bytes())?;
        self.write_all(b" ")?;
        self.write_all(path.as_bytes())?;
        self.write_all(b" ")?;
        self.write_all(version.as_bytes())?;
        self.write_all(b"\r\n")?;
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
        self.write_all(version.as_bytes())?;
        self.write_all(b" ")?;
        self.write_all(status.as_bytes())?;
        self.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes a `Headers` map to a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if a problem was encountered while writing the
    /// `Headers` to the underlying `TcpStream`.
    pub fn write_headers(&mut self, headers: &Headers) -> NetResult<()> {
        self.write_all(headers.to_string().as_bytes())?;
        self.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes a message `Body` to a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the `Body` could not be written
    /// to the underlying `TcpStream` successfully.
    pub fn write_body(&mut self, body: &Body) -> NetResult<()> {
        if !body.is_empty() {
            self.write_all(body.as_bytes())?;
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
        // Ensure the Host header is set.
        if !req.headers.contains(&HOST) {
            let stream = self.writer.get_ref();
            let remote = stream.peer_addr()?;
            req.headers.host(remote);
        }

        self.write_request_line(&req.method, &req.path, &req.version)?;
        self.write_headers(&req.headers)?;
        self.write_body(&req.body)?;
        self.flush()?;
        Ok(())
    }

    /// Writes a `Response` to a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to write any of the
    /// individual components of the `Response` to the `TcpStream`.
    pub fn send_response(&mut self, res: &mut Response) -> NetResult<()> {
        // Ensure Server header is set.
        if !res.headers.contains(&SERVER) {
            res.headers.server(DEFAULT_NAME);
        }

        self.write_status_line(&res.version, &res.status)?;
        self.write_headers(&res.headers)?;
        self.write_body(&res.body)?;
        self.flush()?;
        Ok(())
    }

    /// Writes an internal server error `Response` to a `TcpStream`
    /// that contains the provided error message.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to write any of the
    /// individual components of the `Response` to the `TcpStream`.
    pub fn send_500_error(&mut self, err_msg: String) -> NetResult<()> {
        let body: Body = err_msg.into();

        let res = Response::builder()
            .status_code(500)
            .header("Connection", "close")
            .header("Server", DEFAULT_NAME)
            .header("Cache-Control", "no-cache")
            .header("Content-Length", &body.len().to_string())
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(body)
            .build()?;

        self.write_status_line(&res.version, &res.status)?;
        self.write_headers(&res.headers)?;
        self.write_body(&res.body)?;
        self.flush()?;
        Ok(())
    }
}

/// A trait containing methods for printing CLI argument errors.
pub trait WriteCliError {
    /// Prints unknown option error message and exits the program.
    fn unknown_opt(&self, name: &str) {
        eprintln!("{BR_RED}Unknown option: `{name}`{CLR}");
        process::exit(1);
    }

    /// Prints unknown argument error message and exits the program.
    fn unknown_arg(&self, name: &str) {
        eprintln!("{BR_RED}Unknown argument: `{name}`{CLR}");
        process::exit(1);
    }

    /// Prints missing argument error message and exits the program.
    fn missing_arg(&self, name: &str) {
        eprintln!("{BR_RED}Missing `{name}` argument.{CLR}");
        process::exit(1);
    }

    /// Prints invalid argument error message and exits the program.
    fn invalid_arg(&self, name: &str, arg: &str) {
        eprintln!("{BR_RED}Invalid `{name}` argument: \"{arg}\"{CLR}");
        process::exit(1);
    }
}
