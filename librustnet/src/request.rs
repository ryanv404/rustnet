use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{
    BufRead, BufReader, ErrorKind as IoErrorKind, Read, Result as IoResult,
};
use std::net::TcpStream;
use std::str;
use std::string::ToString;

use crate::consts::{
    CONTENT_LENGTH, CONTENT_TYPE, MAX_HEADERS, READER_BUFSIZE,
};
use crate::{
    Body, HeaderName, HeaderValue, Header, Headers, Method, NetError,
    NetResult, NetWriter, ParseErrorKind, Response, Route, StatusLine,
    Version,
};

/// A buffered reader wrapper around a `TcpStream` instance.
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
    /// Returns a clone of the current `NetReader` instance.
    pub fn try_clone(&self) -> NetResult<Self> {
        let stream = self.0.get_ref().try_clone()?;
        Ok(Self::from(stream))
    }

    /// Consumes the `NetReader` and returns the underlying `TcpStream`.
    pub fn into_inner(self) -> TcpStream {
        self.0.into_inner()
    }

    /// Returns a reference to the underlying `TcpStream`.
    pub fn get_ref(&self) -> &TcpStream {
        self.0.get_ref()
    }

    /// Reads an HTTP request from the underlying `TcpStream`.
    pub fn recv_request(mut reader: Self) -> NetResult<Request> {
        let request_line = reader.read_request_line()?;
        let headers = reader.read_headers()?;
        let body = reader.read_body(&headers)?;
        let reader = Some(reader);

        Ok(Request { request_line, headers, body, reader })
    }

    /// Reads an HTTP response from the underlying `TcpStream`.
    pub fn recv_response(mut reader: Self) -> NetResult<Response> {
        let status_line = reader.read_status_line()?;
        let headers = reader.read_headers()?;
        let body = reader.read_body(&headers)?;
        let writer = Some(NetWriter::from(reader));

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

/// Represents the first line of an HTTP request.
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
            version: Version::OneDotOne
        }
    }
}
impl Display for RequestLine {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {} {}", &self.method, &self.path, &self.version)
    }
}

impl RequestLine {
    /// Returns a new `RequestLine` instance.
    #[must_use]
    pub const fn new(method: Method, path: String, version: Version) -> Self {
        Self { method, path, version }
    }

    /// Returns the HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.method
    }

    /// Returns the URI path to the target resource.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the `Route` representation of the target resource.
    #[must_use]
    pub fn route(&self) -> Route {
        Route::new(self.method, &self.path)
    }

    /// Returns the HTTP version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.version
    }

    /// Parses a string slice into a `RequestLine` object.
    pub fn parse(line: &str) -> NetResult<Self> {
        let mut tokens = line.trim_start().splitn(3, ' ');

        let method = Method::parse(tokens.next())?;

        let path = tokens.next()
            .ok_or(ParseErrorKind::RequestLine)
            .map(ToString::to_string)?;

        let version = Version::parse(tokens.next())?;

        Ok(Self { method, path, version })
    }
}

/// Represents the components of an HTTP request.
pub struct Request {
    pub request_line: RequestLine,
    pub headers: Headers,
    pub body: Body,
    pub reader: Option<NetReader>,
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		// The request line.
		writeln!(f, "{}", self.request_line)?;

		// The request headers.
		for (name, value) in &self.headers.0 {
			writeln!(f, "{name}: {value}")?;
		}

        // The request body.
        if !self.body.is_empty() {
            writeln!(f, "{}", &self.body)?;
        }

		Ok(())
    }
}

impl Debug for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
		f.debug_struct("Request")
			.field("request_line", &self.request_line)
			.field("headers", &self.headers)
            .field("body", &self.body)
			.field("reader", &self.reader)
            .finish()
    }
}

impl Request {
    /// Returns the HTTP method.
    #[must_use]
    pub const fn method(&self) -> Method {
        self.request_line.method
    }

    /// Returns the URI path to the target resource.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.request_line.path
    }

    /// Returns the `Route` representation of the target resource.
    #[must_use]
    pub fn route(&self) -> Route {
        self.request_line.route()
    }

    /// Returns the HTTP version.
    #[must_use]
    pub const fn version(&self) -> Version {
        self.request_line.version
    }

    /// Returns the request line as a String.
    #[must_use]
    pub fn request_line(&self) -> String {
        self.request_line.to_string()
    }

    /// Returns a reference to the request headers.
    #[must_use]
    pub const fn headers(&self) -> &Headers {
        &self.headers
    }

    /// Returns true if the header is present.
    #[must_use]
    pub fn has_header(&self, name: &HeaderName) -> bool {
        self.headers.contains(name)
    }

    /// Returns the header value for the given `HeaderName`, if present.
    #[must_use]
    pub fn get_header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

	/// Adds or updates a request header field line.
    pub fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    /// Returns the request headers as a String.
    #[must_use]
    pub fn headers_to_string(&self) -> String {
        if self.headers.0.is_empty() {
            String::new()
        } else {
            self.headers.0.iter().fold(String::new(), 
                |mut acc, (name, value)| {
                    acc.push_str(&format!("{name}: {value}\n"));
                    acc
                })
        }
    }

	/// Returns a reference to the request body, if present.
	#[must_use]
	pub const fn body(&self) -> &Body {
		&self.body
	}

    /// Sends an HTTP request to a remote server.
    pub fn send(&mut self) -> NetResult<()> {
        let mut writer = self.reader
            .take()
            .and_then(|reader| Some(NetWriter::from(reader)))
            .ok_or_else(|| IoErrorKind::NotConnected)?;

        writer.send_request(self)
    }

    /// Receives an HTTP request from a remote client.
    pub fn recv(reader: NetReader) -> NetResult<Request> {
        NetReader::recv_request(reader)
    }
}
