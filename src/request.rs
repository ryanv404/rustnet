use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{BufRead, BufReader, Read, Result as IoResult};
use std::net::TcpStream;
use std::str::{self, FromStr};
use std::string::ToString;

use crate::{
    Body, Header, HeaderName, HeaderValue, Headers, Method, NetError,
    NetParseError, NetResult, NetWriter, Response, Route, StatusLine, Version,
    READER_BUFSIZE,
};
use crate::header::MAX_HEADERS;

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
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`. An error will also
    /// be returned if parsing of the `RequestLine` fails.
    pub fn read_request_line(&mut self) -> NetResult<RequestLine> {
        let mut line = String::with_capacity(1024);

        match self.read_line(&mut line) {
            Err(e) => Err(NetError::Read(e.kind())),
            Ok(0) => Err(NetError::UnexpectedEof),
            Ok(_) => line.parse::<RequestLine>(),
        }
    }

    /// Reads and parses a `StatusLine` from the underlying `TcpStream`.
    ///
    /// # Errors
    ///
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`. An error will also
    /// be returned if parsing of the `StatusLine` fails.
    pub fn read_status_line(&mut self) -> NetResult<StatusLine> {
        let mut line = String::with_capacity(1024);

        match self.read_line(&mut line) {
            Err(e) => Err(NetError::Read(e.kind())),
            Ok(0) => Err(NetError::UnexpectedEof),
            Ok(_) => line.parse::<StatusLine>(),
        }
    }

    /// Reads and parses header entries from the underlying `TcpStream`
    /// into a `Headers` collection.
    ///
    /// # Errors
    ///
    /// As with the other readers, an error of kind `NetError::UnexpectedEof`
    /// is returned if `Ok(0)` is received while reading from the underlying
    /// `TcpStream`. An error will also be returned if parsing of a `Header`
    /// entry fails.
    pub fn read_headers(&mut self) -> NetResult<Headers> {
        let mut num_headers = 0;
        let mut headers = Headers::new();
        let mut line = String::with_capacity(1024);

        while num_headers <= MAX_HEADERS {
            num_headers += 1;

            line.clear();

            match self.read_line(&mut line) {
                Err(e) => return Err(NetError::Read(e.kind())),
                Ok(0) => return Err(NetError::UnexpectedEof),
                Ok(_) => {
                    let trimmed_line = line.trim();

                    // Check for end of headers section.
                    if trimmed_line.is_empty() {
                        break;
                    }

                    trimmed_line
                        .parse::<Header>()
                        .map(|hdr| headers.insert(hdr.name, hdr.value))?;
                },
            }
        }

        Ok(headers)
    }

    /// Reads and parses a `Body` from the underlying `TcpStream`, if present.
    ///
    /// # Errors
    ///
    /// An error of kind `NetError::UnexpectedEof` is returned if an attempt
    /// to read the underlying `TcpStream` returns `Ok(0)`.
    pub fn read_body(&mut self, headers: &Headers) -> NetResult<Body> {
        use crate::header::{CONTENT_LENGTH, CONTENT_TYPE};

        let content_len = headers.get(&CONTENT_LENGTH);
        let content_type = headers.get(&CONTENT_TYPE);

        if content_len.is_none() || content_type.is_none() {
            return Ok(Body::Empty);
        }

        let body_len = content_len
            .ok_or(NetError::Parse(NetParseError::Body))
            .map(ToString::to_string)
            .and_then(|s| {
                s.trim()
                    .parse::<usize>()
                    .map_err(|_| NetError::Parse(NetParseError::Body))
            })?;

        if body_len == 0 {
            return Ok(Body::Empty);
        }

        let num_bytes = u64::try_from(body_len)
            .map_err(|_| NetError::Parse(NetParseError::Body))?;

        let body_type = content_type
            .map(ToString::to_string)
            .ok_or(NetError::Parse(NetParseError::Body))?;

        if body_type.is_empty() {
            // Return error since content length is greater than zero.
            return Err(NetError::Parse(NetParseError::Body));
        }

        let mut reader = self.take(num_bytes);
        let mut buf = Vec::with_capacity(body_len);

        reader.read_to_end(&mut buf)?;

        let mut type_tokens = body_type.splitn(2, '/');

        match type_tokens.next().map(str::trim) {
            Some("text") => match type_tokens.next().map(str::trim) {
                Some(s) if s.starts_with("html") => Ok(Body::Html(buf)),
                Some(s) if s.starts_with("plain") => Ok(Body::Text(buf)),
                _ => Ok(Body::Text(buf)),
            },
            Some("application") => match type_tokens.next().map(str::trim) {
                Some(s) if s.starts_with("json") => Ok(Body::Json(buf)),
                Some(s) if s.starts_with("xml") => Ok(Body::Xml(buf)),
                _ => Ok(Body::Bytes(buf)),
            },
            Some("image") => match type_tokens.next().map(str::trim) {
                Some(s) if s.starts_with("x-icon") => Ok(Body::Favicon(buf)),
                _ => Ok(Body::Bytes(buf)),
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
            .ok_or(NetError::Parse(NetParseError::Method))
            .and_then(str::parse)?;

        let path = tokens
            .next()
            .map(ToString::to_string)
            .ok_or(NetError::Parse(NetParseError::UriPath))?;

        let version = tokens
            .next()
            .ok_or(NetError::Parse(NetParseError::Version))
            .and_then(str::parse)?;

        Ok(Self {
            method,
            path,
            version,
        })
    }
}

impl RequestLine {
    /// Returns a new `RequestLine` instance from the provided HTTP method
    /// and URI path.
    #[must_use]
    pub fn new(method: Method, path: &str) -> Self {
        Self {
            method,
            path: path.to_string(),
            version: Version::OneDotOne,
        }
    }

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

    // Common logic for the to_plain_string and to_color_string functions.
    fn string_helper(&self, use_color: bool) -> String {
        const YLW: &str = "\x1b[95m";
        const CLR: &str = "\x1b[0m";

        if use_color {
            format!(
                "{YLW}{} {} {}{CLR}\n",
                &self.method,
                &self.path,
                &self.version
            )
        } else {
            format!("{self}\n")
        }
    }

    /// Returns the `RequestLine` as a `String` with plain formatting.
    #[must_use]
    pub fn to_plain_string(&self) -> String {
        self.string_helper(false)
    }

    /// Returns the `RequestLine` as a `String` with color formatting.
    #[must_use]
    pub fn to_color_string(&self) -> String {
        self.string_helper(true)
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
        writeln!(f, "{}", &self.request_line)?;

        for (name, value) in &self.headers.0 {
            writeln!(f, "{name}: {value}")?;
        }

        if self.body.is_printable() {
            let body = String::from_utf8_lossy(self.body.as_bytes());
            writeln!(f, "{body}")?;
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

    /// Returns the requested `Route`.
    #[must_use]
    pub fn route(&self) -> Route {
        Route::new(self.method(), self.path())
    }

    /// Returns the `RequestLine` for this `Request`.
    #[must_use]
    pub const fn request_line(&self) -> &RequestLine {
        &self.request_line
    }

    /// Returns the headers found in this `Request`.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the given `HeaderName` key is present.
    #[must_use]
    pub fn contains(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Adds or updates a header field line to this `Request`.
    pub fn header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
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
