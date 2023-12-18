use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{
    BufRead, BufReader, ErrorKind as IoErrorKind, Read, Result as IoResult,
};
use std::net::TcpStream;
use std::str::{self, FromStr};
use std::string::ToString;

use crate::{
    Body, Header, HeaderName, HeaderValue, Headers, Method, NetError,
    NetResult, NetWriter, ParseErrorKind, Response, Route, StatusLine,
    Version, MAX_HEADERS, READER_BUFSIZE,
};

/// A buffered reader responsible for reading from an inner `TcpStream`.
#[derive(Debug)]
pub struct NetReader(pub BufReader<TcpStream>);

impl From<TcpStream> for NetReader {
    fn from(stream: TcpStream) -> Self {
        Self(BufReader::with_capacity(READER_BUFSIZE, stream))
    }
}

impl From<NetWriter> for NetReader {
    fn from(writer: NetWriter) -> Self {
        Self::from(writer.0.into_parts().0)
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
    /// Returns a clone of the current `NetReader`.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying call to `TcpStream::try_clone`
    /// encounters an error.
    pub fn try_clone(&self) -> NetResult<Self> {
        let stream = self.get_ref().try_clone()?;
        Ok(Self::from(stream))
    }

    /// Consumes the `NetReader` and returns the underlying `TcpStream`.
    #[must_use]
    pub fn into_inner(self) -> TcpStream {
        self.0.into_inner()
    }

    /// Returns a reference to the underlying `TcpStream`.
    #[must_use]
    pub fn get_ref(&self) -> &TcpStream {
        self.0.get_ref()
    }

    /// Reads and parses a `RequestLine` from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `ErrorKind::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`. An error will also
    /// be returned if parsing of the `RequestLine` fails.
    pub fn read_request_line(&mut self) -> NetResult<RequestLine> {
        let mut line = String::with_capacity(1024);

        match self.read_line(&mut line) {
            Ok(0) => Err(IoErrorKind::UnexpectedEof.into()),
            Err(e) => Err(NetError::ReadError(e.kind())),
            Ok(_) => line.parse::<RequestLine>(),
        }
    }

    /// Reads and parses a `StatusLine` from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `ErrorKind::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`. An error will also
    /// be returned if parsing of the `StatusLine` fails.
    pub fn read_status_line(&mut self) -> NetResult<StatusLine> {
        let mut line = String::with_capacity(1024);

        match self.read_line(&mut line) {
            Ok(0) => Err(IoErrorKind::UnexpectedEof.into()),
            Err(e) => Err(NetError::ReadError(e.kind())),
            Ok(_) => line.parse::<StatusLine>(),
        }
    }

    /// Reads and parses header entries from the underlying `TcpStream`
    /// into a `Headers` collection.
    ///
    /// # Errors
    ///
    /// As with the other readers, an error of kind `ErrorKind::UnexpectedEof`
    /// is returned if `Ok(0)` is received while reading from the underlying
    /// `TcpStream`. An error will also be returned if parsing of a `Header`
    /// entry fails.
    pub fn read_headers(&mut self) -> NetResult<Headers> {
        let mut num_headers = 0;
        let mut headers = Headers::new();
        let mut line = String::with_capacity(1024);

        while num_headers <= MAX_HEADERS {
            line.clear();

            match self.read_line(&mut line) {
                Ok(0) => return Err(IoErrorKind::UnexpectedEof)?,
                Err(e) => return Err(NetError::ReadError(e.kind())),
                Ok(_) => {
                    let buf = line.trim();

                    if buf.is_empty() {
                        break;
                    }

                    let header = buf.parse::<Header>()?;
                    headers.insert(header.name, header.value);
                    num_headers += 1;
                }
            }
        }

        Ok(headers)
    }

    /// Reads and parses a `Body` from the underlying `TcpStream`, if present.
    ///
    /// # Errors
    ///
    /// An error of kind `ErrorKind::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`.
    pub fn read_body(&mut self, headers: &Headers) -> NetResult<Body> {
        use crate::header::{CONTENT_LENGTH, CONTENT_TYPE};

        let content_len = headers.get(&CONTENT_LENGTH);
        let content_type = headers.get(&CONTENT_TYPE);

        if content_len.is_none() || content_type.is_none() {
            return Ok(Body::Empty);
        }

        let body_len = content_len
            .ok_or(ParseErrorKind::Body)
            .map(ToString::to_string)
            .and_then(|s| s.trim().parse::<usize>()
                .map_err(|_| ParseErrorKind::Body))?;

        if body_len == 0 {
            return Ok(Body::Empty);
        }

        let num_bytes = u64::try_from(body_len)
            .map_err(|_| ParseErrorKind::Body)?;

        let body_type = content_type
            .map(ToString::to_string)
            .ok_or(ParseErrorKind::Body)?;

        if body_type.is_empty() {
            // Return error since content length is greater than zero.
            return Err(ParseErrorKind::Body)?;
        }

        let mut reader = self.take(num_bytes);
        let mut buf = Vec::with_capacity(body_len);
        reader.read_to_end(&mut buf)?;

        let mut type_tokens = body_type.splitn(2, '/');

        match type_tokens.next().map(str::trim) {
            Some("text") => match type_tokens.next().map(str::trim) {
                Some(s) if s.starts_with("html") => Ok(Body::Text(buf)),
                Some(s) if s.starts_with("plain") => Ok(Body::Text(buf)),
                _ => Ok(Body::Text(buf)),
            },
            Some("application") => match type_tokens.next().map(str::trim) {
                Some(s) if s.starts_with("json") => Ok(Body::Json(buf)),
                Some(s) if s.starts_with("xml") => Ok(Body::Xml(buf)),
                Some(s) if s.starts_with("octet-stream") => {
                    Ok(Body::Bytes(buf))
                },
                _ => Ok(Body::Bytes(buf)),
            },
            Some("image") => match type_tokens.next().map(str::trim) {
                Some(s) if s.starts_with("x-icon") => Ok(Body::Favicon(buf)),
                Some(s) if s.starts_with("png") => Ok(Body::Image(buf)),
                Some(s) if s.starts_with("jpeg") => Ok(Body::Image(buf)),
                Some(s) if s.starts_with("gif") => Ok(Body::Image(buf)),
                _ => Ok(Body::Image(buf)),
            },
            _ => Ok(Body::Bytes(buf)),
        }
    }

    /// Reads and parses a `Request` from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to read or parse the
    /// individual components of the `Request`.
    pub fn recv_request(&mut self) -> NetResult<Request> {
        let request_line = self.read_request_line()?;
        let headers = self.read_headers()?;
        let body = self.read_body(&headers)?;

        Ok(Request {
            request_line,
            headers,
            body,
        })
    }

    /// Reads and parses a `Response` from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error is returned if there is a failure to read or parse the
    /// individual components of the `Response`.
    pub fn recv_response(&mut self) -> NetResult<Response> {
        let status_line = self.read_status_line()?;
        let headers = self.read_headers()?;
        let body = self.read_body(&headers)?;

        Ok(Response {
            status_line,
            headers,
            body,
        })
    }
}

/// Contains the components of an HTTP request line.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestLine {
    pub method: Method,
    pub path: String,
    pub version: Version,
}

impl Default for RequestLine {
    fn default() -> Self {
        Self {
            method: Method::Get,
            path: String::from("/"),
            version: Version::OneDotOne,
        }
    }
}

impl Display for RequestLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {} {}", &self.method, &self.path, &self.version)
    }
}

impl FromStr for RequestLine {
    type Err = NetError;

    /// Parses a string slice into a `RequestLine`.
    ///
    /// # Errors
    ///
    /// An error is returned if a problem is encountered while parsing
    /// the HTTP `Method`, URI path, or HTTP protocol `Version` that
    /// together comprise the `RequestLine`.
    fn from_str(line: &str) -> NetResult<Self> {
        let mut tokens = line.trim_start().splitn(3, ' ');

        let method = tokens
            .next()
            .ok_or(NetError::ParseError(ParseErrorKind::RequestLine))
            .and_then(str::parse)?;

        let path = tokens
            .next()
            .map(ToString::to_string)
            .ok_or(ParseErrorKind::RequestLine)?;

        let version = tokens
            .next()
            .ok_or(NetError::ParseError(ParseErrorKind::RequestLine))
            .and_then(str::parse)?;

        Ok(Self {
            method,
            path,
            version,
        })
    }
}

impl RequestLine {
    /// Returns the HTTP `Method` for this `RequestLine`.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the requested URI path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the HTTP protocol `Version`.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }
}

/// Contains the components of an HTTP request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Request {
    pub request_line: RequestLine,
    pub headers: Headers,
    pub body: Body,
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        writeln!(f, "{}", self.request_line)?;

        for (name, value) in &self.headers.0 {
            writeln!(f, "{name}: {value}")?;
        }

        if !self.body.is_empty() {
            writeln!(f, "{}", &self.body)?;
        }

        Ok(())
    }
}

impl Request {
    /// Returns the HTTP `Method` for this `Request`.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.request_line.method
    }

    /// Returns the URI path for this `Request`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.request_line.path
    }

    /// Returns the HTTP protocol `Version` for this `Request`.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.request_line.version
    }

    /// Returns a `Route` that represents the URI path and HTTP
    /// `Method` found in this `Request`.
    #[must_use]
    pub fn route(&self) -> Route {
        Route::from((self.method(), self.path()))
    }

    /// Returns the `RequestLine` for this `Request`.
    #[must_use]
    pub fn request_line(&self) -> String {
        self.request_line.to_string()
    }

    /// Returns the headers found in this `Request`.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the given `HeaderName` key is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Returns the header value for the given `HeaderName`, if present.
    #[must_use]
    pub fn get_header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Adds or updates a header field line to this `Request`.
    pub fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    /// Returns the all request headers as a `String`.
    #[must_use]
    pub fn headers_to_string(&self) -> String {
        if self.headers.0.is_empty() {
            String::new()
        } else {
            self.headers
                .0
                .iter()
                .fold(String::new(), |mut acc, (name, value)| {
                    acc.push_str(&format!("{name}: {value}\n"));
                    acc
                })
        }
    }

    /// Returns `Body` for this `Request`.
    #[must_use]
    pub const fn body(&self) -> &Body {
        &self.body
    }

    /// Writes an HTTP request to a remote server.
    ///
    /// # Errors
    ///
    /// An error is returned if `NetWriter::send_request` encounters an
    /// error.
    pub fn send(&mut self, writer: &mut NetWriter) -> NetResult<()> {
        writer.send_request(self)
    }

    /// Reads and parses an HTTP request from a remote client.
    ///
    /// # Errors
    ///
    /// An error is returned if `NetReader::recv_request` encounters an
    /// error.
    pub fn recv(reader: &mut NetReader) -> NetResult<Self> {
        reader.recv_request()
    }
}
