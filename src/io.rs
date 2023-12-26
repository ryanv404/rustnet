use std::borrow::Cow;
use std::fmt::Debug;
use std::io::{
    BufRead, BufReader, BufWriter, Read, Result as IoResult, Write,
};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::process;
use std::str;

use crate::{
    Body, Headers, NetError, NetParseError, NetResult, Request, RequestLine,
    Response, StatusLine, READER_BUFSIZE, WRITER_BUFSIZE,
};
use crate::colors::{CLR, RED};
use crate::header::MAX_HEADERS;
use crate::header_name::{
    ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, HOST, SERVER, USER_AGENT,
};
use crate::util;

/// Represents the TCP connection between a client and a server.
#[derive(Debug)]
pub struct Connection {
    pub reader: BufReader<TcpStream>,
    pub writer: BufWriter<TcpStream>,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
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

    /// Reads and parses a `RequestLine` from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`.
    pub fn recv_request_line(
        &mut self,
        buf: &mut Vec<u8>
    ) -> NetResult<RequestLine> {
        match self.read_until(b'\n', buf) {
            Ok(0) => Err(NetError::UnexpectedEof),
            Ok(_) => RequestLine::try_from(&buf[..]),
            Err(e) => Err(NetError::Read(e.kind())),
        }
    }

    /// Reads and parses a `StatusLine` from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`.
    pub fn recv_status_line(
        &mut self,
        buf: &mut Vec<u8>
    ) -> NetResult<StatusLine> {
        match self.read_until(b'\n', buf) {
            Ok(0) => Err(NetError::UnexpectedEof),
            Ok(_) => StatusLine::try_from(&buf[..]),
            Err(e) => Err(NetError::Read(e.kind())),
        }
    }

    /// Reads all headers from the underlying `TcpStream` into the provided
    /// `String` buffer returning the total number of headers read.
    ///
    /// # Errors
    ///
    /// As with the other readers, an error of kind `NetError::UnexpectedEof`
    /// is returned if `Ok(0)` is received while reading from the underlying
    /// `TcpStream`.
    pub fn recv_headers(
        &mut self,
        buf: &mut Vec<u8>
    ) -> NetResult<Headers> {
        let mut num_headers = 0;

        let mut headers = Headers::new();

        loop {
            if num_headers >= MAX_HEADERS {
                return Err(NetParseError::TooManyHeaders)?;
            }

            buf.clear();

            match self.read_until(b'\n', buf) {
                Err(e) => Err(NetError::Read(e.kind()))?,
                Ok(0) => Err(NetError::UnexpectedEof)?,
                Ok(_) => {
                    let trimmed = util::trim_bytes(&buf[..]);

                    if trimmed.is_empty() {
                        break;
                    }

                    headers.insert_parsed_header_bytes(trimmed)?;
                },
            }

            num_headers += 1;
        }

        Ok(headers)
    }

    /// Reads a message `Body` from the underlying `TcpStream`.
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
        if content_len == 0 {
            return Ok(Body::Empty);
        }

        let mut reader = self.take(content_len);

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
        let mut buf = Vec::with_capacity(1024);

        let request_line = self.recv_request_line(&mut buf)?;
        buf.clear();

        let headers = self.recv_headers(&mut buf)?;
        buf.clear();

        let content_len = headers
            .get(&CONTENT_LENGTH)
            .map(|value| value.as_str())
            .and_then(|len| len.trim().parse::<u64>().ok())
            .unwrap_or(0);

        let content_type = headers
            .get(&CONTENT_TYPE)
            .map_or(Cow::Borrowed(""), |value| value.as_str());

        let body = self.recv_body(
            &mut buf,
            content_len,
            &content_type
        )?;

        Ok(Request {request_line, headers, body })
    }

    /// Reads and parses a `Response` from a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to read or parse the
    /// individual components of the `Response`.
    pub fn recv_response(&mut self) -> NetResult<Response> {
        let mut buf = Vec::with_capacity(1024);

        let status_line = self.recv_status_line(&mut buf)?;
        buf.clear();

        let headers = self.recv_headers(&mut buf)?;
        buf.clear();

        let content_len = headers
            .get(&CONTENT_LENGTH)
            .map(|value| value.as_str())
            .and_then(|len| len.trim().parse::<u64>().ok())
            .unwrap_or(0);

        let content_type = headers
            .get(&CONTENT_TYPE)
            .map_or(Cow::Borrowed(""), |value| value.as_str());

        let body = self.recv_body(
            &mut buf,
            content_len,
            &content_type
        )?;

        Ok(Response { status_line, headers, body })
    }

    /// Writes a `RequestLine` to a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the `RequestLine` could not be written
    /// to the underlying `TcpStream` successfully.
    pub fn write_request_line(
        &mut self,
        request_line: &RequestLine,
    ) -> NetResult<()> {
        self.write_all(request_line.to_string().as_bytes())?;
        self.write_all(b"\r\n")?;
        Ok(())
    }

    /// Writes a `StatusLine` to a `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if the `StatusLine` could not be written
    /// to the underlying `TcpStream` successfully.
    pub fn write_status_line(
        &mut self,
        status_line: &StatusLine,
    ) -> NetResult<()> {
        self.write_all(status_line.to_string().as_bytes())?;
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
        if !req.headers.contains(&ACCEPT) {
            req.headers.accept("*/*");
        }

        if !req.headers.contains(&HOST) {
            let stream = self.writer.get_ref();
            let remote = stream.peer_addr()?;
            req.headers.host(&remote);
        }

        if !req.headers.contains(&USER_AGENT) {
            req.headers.user_agent("rustnet/0.1");
        }

        self.write_request_line(&req.request_line)?;
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
        if !res.headers.contains(&SERVER) {
            res.headers.server("rustnet/0.1");
        }

        self.write_status_line(&res.status_line)?;
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
    pub fn send_500_error(&mut self, err_msg: &str) -> NetResult<()> {
        let mut res = Response::try_from(500)?;

        // Include the provided error message.
        res.body = Body::Text(err_msg.into());

        // Update the response headers.
        res.headers.connection("close");
        res.headers.server("rustnet/0.1");
        res.headers.cache_control("no-cache");
        res.headers.content_length(res.body.len());
        res.headers.content_type("text/plain; charset=utf-8");

        self.write_status_line(&res.status_line)?;
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
        eprintln!("{RED}Unknown option: `{name}`{CLR}");
        process::exit(1);
    }

    /// Prints unknown argument error message and exits the program.
    fn unknown_arg(&self, name: &str) {
        eprintln!("{RED}Unknown argument: `{name}`{CLR}");
        process::exit(1);
    }

    /// Prints missing argument error message and exits the program.
    fn missing_arg(&self, name: &str) {
        eprintln!("{RED}Missing `{name}` argument.{CLR}");
        process::exit(1);
    }

    /// Prints invalid argument error message and exits the program.
    fn invalid_arg(&self, name: &str, arg: &str) {
        eprintln!("{RED}Invalid `{name}` argument: \"{arg}\"{CLR}");
        process::exit(1);
    }
}
