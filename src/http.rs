use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::{self, FromStr};

use crate::NetParseError;

/// HTTP methods.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Method {
    /// Transfers a current representation of the target resource.
    Get,
    /// Performs resource-specific processing on the request content.
    Post,
    /// Replaces all current representations of the target resource with the
    /// request content.
    Put,
    /// Performs a similar action to PUT but can do partial updates.
    Patch,
    /// Removes all current representations of the target resource.
    Delete,
    /// Performs the same action as GET but does not transfer the response
    /// content.
    Head,
    /// Performs a message loop-back test along the target resource path.
    Trace,
    /// Describes the communication options for the target resource.
    Options,
    /// Establishes a tunnel to the server identified by the target resource.
    Connect,
    /// Custom method to shut down a test server.
    Shutdown,
}

impl Default for Method {
    fn default() -> Self {
        Self::Get
    }
}

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Method {
    type Err = NetParseError;

    fn from_str(method: &str) -> Result<Self, Self::Err> {
        match method {
            // HTTP methods are case-sensitive.
            "GET" => Ok(Self::Get),
            "PUT" => Ok(Self::Put),
            "POST" => Ok(Self::Post),
            "HEAD" => Ok(Self::Head),
            "PATCH" => Ok(Self::Patch),
            "TRACE" => Ok(Self::Trace),
            "DELETE" => Ok(Self::Delete),
            "OPTIONS" => Ok(Self::Options),
            "CONNECT" => Ok(Self::Connect),
            "SHUTDOWN" => Ok(Self::Shutdown),
            _ => Err(NetParseError::Method),
        }
    }
}

impl TryFrom<&[u8]> for Method {
    type Error = NetParseError;

    fn try_from(method: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(method)
            .map_err(|_| NetParseError::Method)
            .and_then(Self::from_str)
    }
}

impl Method {
    /// Returns the HTTP `Method` as a bytes slice.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Get => b"GET",
            Self::Put => b"PUT",
            Self::Post => b"POST",
            Self::Head => b"HEAD",
            Self::Patch => b"PATCH",
            Self::Trace => b"TRACE",
            Self::Delete => b"DELETE",
            Self::Options => b"OPTIONS",
            Self::Connect => b"CONNECT",
            Self::Shutdown => b"SHUTDOWN",
        }
    }

    /// Returns the HTTP `Method` as a copy-on-write string slice.
    #[must_use]
    pub fn as_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }
}

/// HTTP response status.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Status(pub u16);

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {}", self.code(), self.msg())
    }
}

impl Default for Status {
    fn default() -> Self {
        Self(200u16)
    }
}

impl FromStr for Status {
    type Err = NetParseError;

    fn from_str(status: &str) -> Result<Self, Self::Err> {
        let Some((code, _)) = status.split_once(' ') else {
            return Err(NetParseError::Status);
        };

        u16::from_str(code)
            .map_err(|_| NetParseError::Status)
            .and_then(Self::try_from)
    }
}

impl TryFrom<&[u8]> for Status {
    type Error = NetParseError;

    fn try_from(status: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(status)
            .map_err(|_| NetParseError::Status)
            .and_then(Self::from_str)
    }
}

impl TryFrom<u16> for Status {
    type Error = NetParseError;

    fn try_from(code: u16) -> Result<Self, Self::Error> {
        if matches!(code, 100..=999) {
            Ok(Self(code))
        } else {
            Err(NetParseError::Status)
        }
    }
}

impl TryFrom<u32> for Status {
    type Error = NetParseError;

    fn try_from(code: u32) -> Result<Self, Self::Error> {
        u16::try_from(code)
            .map_err(|_| NetParseError::Status)
            .and_then(Self::try_from)
    }
}

impl TryFrom<i32> for Status {
    type Error = NetParseError;

    fn try_from(code: i32) -> Result<Self, Self::Error> {
        u16::try_from(code)
            .map_err(|_| NetParseError::Status)
            .and_then(Self::try_from)
    }
}

impl Status {
    /// Returns the `Status` as a bytes slice.
    #[must_use]
    pub const fn as_bytes(&self) -> &'static [u8] {
        match self.0 {
            // 1xx informational status codes.
            100..=199 => self.get_1xx_status_msg(),
            // 2xx success status codes.
            200..=299 => self.get_2xx_status_msg(),
            // 3xx redirect status codes.
            300..=399 => self.get_3xx_status_msg(),
            // 4xx client error status codes.
            400..=499 => self.get_4xx_status_msg(),
            // 5xx server error status codes.
            500..=599 => self.get_5xx_status_msg(),
            999 => b"999 Request Denied",
            // Unrecognized status codes.
            _ => b"",
        }
    }

    /// Returns the status reason phrase for the given 1xx status code.
    #[must_use]
    pub const fn get_1xx_status_msg(&self) -> &'static [u8] {
        match self.0 {
            // 1xx informational status codes.
            100 => b"100 Continue",
            101 => b"101 Switching Protocols",
            102 => b"102 Processing",
            103 => b"103 Early Hints",
            _ => b"",
        }
    }

    /// Returns the status reason phrase for the given 2xx status code.
    #[must_use]
    pub const fn get_2xx_status_msg(&self) -> &'static [u8] {
        match self.0 {
            // 2xx success status codes.
            200 => b"200 OK",
            201 => b"201 Created",
            202 => b"202 Accepted",
            203 => b"203 Non-Authoritative Information",
            204 => b"204 No Content",
            205 => b"205 Reset Content",
            206 => b"206 Partial Content",
            207 => b"207 Multi-Status",
            208 => b"208 Already Reported",
            218 => b"218 This Is Fine",
            226 => b"226 IM Used",
            _ => b"",
        }
    }

    /// Returns the status reason phrase for the given 3xx status code.
    #[must_use]
    pub const fn get_3xx_status_msg(&self) -> &'static [u8] {
        match self.0 {
            // 3xx redirect status codes.
            300 => b"300 Multiple Choices",
            301 => b"301 Moved Permanently",
            302 => b"302 Found",
            303 => b"303 See Other",
            304 => b"304 Not Modified",
            305 => b"305 Use Proxy",
            306 => b"306 Switch Proxy",
            307 => b"307 Temporary Redirect",
            308 => b"308 Permanent Redirect",
            _ => b"",
        }
    }

    /// Returns the status reason phrase for the given 4xx status code.
    #[must_use]
    pub const fn get_4xx_status_msg(&self) -> &'static [u8] {
        match self.0 {
            // 4xx client error status codes.
            400 => b"400 Bad Request", // No or multiple Host headers, invalid request line.
            401 => b"401 Unauthorized",
            402 => b"402 Payment Required",
            403 => b"403 Forbidden",
            404 => b"404 Not Found",
            405 => b"405 Method Not Allowed",
            406 => b"406 Not Acceptable",
            407 => b"407 Proxy Authentication Required",
            408 => b"408 Request Timeout",
            409 => b"409 Conflict",
            410 => b"410 Gone",
            411 => b"411 Length Required",
            412 => b"412 Precondition Failed",
            413 => b"413 Payload Too Large",
            414 => b"414 URI Too Long", // Recommended to support 8kb+ request lines.
            415 => b"415 Unsupported Media Type",
            416 => b"416 Range Not Satisfiable",
            417 => b"417 Expectation Failed",
            418 => b"418 I'm a Teapot",
            419 => b"419 Page Expired",
            420 => b"420 Method Failure or Enhance Your Calm",
            421 => b"421 Misdirected Request",
            422 => b"422 Unprocessable Entity",
            423 => b"423 Locked",
            424 => b"424 Failed Dependency",
            425 => b"425 Too Early",
            426 => b"426 Upgrade Required",
            428 => b"428 Precondition Required",
            429 => b"429 Too Many Requests",
            430 => b"430 HTTP Status Code",
            431 => b"431 Request Header Fields Too Large",
            440 => b"440 Login Time-Out",
            444 => b"444 No Response",
            449 => b"449 Retry With",
            450 => b"450 Blocked by Windows Parental Controls",
            451 => b"451 Unavailable For Legal Reasons",
            460 => b"460 Client Closed Connection Prematurely",
            463 => b"463 Too Many Forwarded IP Addresses",
            464 => b"464 Incompatible Protocol",
            494 => b"494 Request Header Too Large",
            495 => b"495 SSL Certificate Error",
            496 => b"496 SSL Certificate Required",
            497 => b"497 HTTP Request Sent to HTTPS Port",
            498 => b"498 Invalid Token",
            499 => b"499 Token Required or Client Closed Request",
            _ => b"",
        }
    }

    /// Returns the status reason phrase for the given 5xx status code.
    #[must_use]
    pub const fn get_5xx_status_msg(&self) -> &'static [u8] {
        match self.0 {
            // 5xx server error status codes.
            500 => b"500 Internal Server Error",
            501 => b"501 Not Implemented", // Unimplemented methods, etc.
            502 => b"502 Bad Gateway",
            503 => b"503 Service Unavailable",
            504 => b"504 Gateway Timeout",
            505 => b"505 HTTP Version Not Supported",
            506 => b"506 Variant Also Negotiates",
            507 => b"507 Insufficient Storage",
            508 => b"508 Loop Detected",
            509 => b"509 Bandwidth Limit Exceeded",
            510 => b"510 Not Extended",
            511 => b"511 Network Authentication Required",
            520 => b"520 Web Server Is Returning an Unknown Error",
            521 => b"521 Web Server Is Down",
            522 => b"522 Connection Timed Out",
            523 => b"523 Origin Is Unreachable",
            524 => b"524 A Timeout Occurred",
            525 => b"525 SSL Handshake Failed",
            526 => b"526 Invalid SSL Certificate",
            527 => b"527 Railgun Listener to Origin",
            529 => b"529 The Service Is Overloaded",
            530 => b"530 Site Frozen",
            561 => b"561 Unauthorized",
            598 => b"598 Network Read Timeout Error",
            599 => b"599 Network Connect Timeout Error",
            _ => b"",
        }
    }

    /// Returns the `Status` as a string slice.
    #[must_use]
    pub fn as_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }

    /// Returns the `Status` reason phrase as a string slice.
    #[must_use]
    pub fn msg(&self) -> Cow<'_, str> {
        match self.as_str().split_once(' ') {
            Some((_, msg)) => msg.to_string().into(),
            None => Cow::Borrowed(""),
        }
    }

    /// Returns the status code.
    #[must_use]
    pub const fn code(&self) -> u16 {
        self.0
    }
}

/// The HTTP protocol version.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Version {
    /// HTTP version 0.9
    ZeroDotNine,
    /// HTTP version 1.0
    OneDotZero,
    /// HTTP version 1.1
    OneDotOne,
    /// HTTP version 2.0
    TwoDotZero,
    /// HTTP version 3.0
    ThreeDotZero,
}

impl Default for Version {
    fn default() -> Self {
        Self::OneDotOne
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Version {
    type Err = NetParseError;

    fn from_str(version: &str) -> Result<Self, Self::Err> {
        match version {
            "HTTP/0.9" => Ok(Self::ZeroDotNine),
            "HTTP/1.0" => Ok(Self::OneDotZero),
            "HTTP/1.1" => Ok(Self::OneDotOne),
            "HTTP/2" | "HTTP/2.0" => Ok(Self::TwoDotZero),
            "HTTP/3" | "HTTP/3.0" => Ok(Self::ThreeDotZero),
            _ => Err(NetParseError::Version),
        }
    }
}

impl TryFrom<(u8, u8)> for Version {
    type Error = NetParseError;

    fn try_from((major, minor): (u8, u8)) -> Result<Self, Self::Error> {
        match (major, minor) {
            (0, 9) => Ok(Self::ZeroDotNine),
            (1, 0) => Ok(Self::OneDotZero),
            (1, 1) => Ok(Self::OneDotOne),
            (2, 0) => Ok(Self::TwoDotZero),
            (3, 0) => Ok(Self::ThreeDotZero),
            _ => Err(NetParseError::Version),
        }
    }
}

impl TryFrom<&[u8]> for Version {
    type Error = NetParseError;

    fn try_from(version: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(version)
            .map_err(|_| NetParseError::Version)
            .and_then(Self::from_str)
    }
}

impl Version {
    /// Returns the the protocol `Version` as a bytes slice.
    #[must_use]
    pub const fn as_bytes(&self) -> &'static [u8] {
        match self {
            Self::ZeroDotNine => b"HTTP/0.9",
            Self::OneDotZero => b"HTTP/1.0",
            Self::OneDotOne => b"HTTP/1.1",
            Self::TwoDotZero => b"HTTP/2.0",
            Self::ThreeDotZero => b"HTTP/3.0",
        }
    }

    /// Returns the the protocol `Version` as a string slice.
    #[must_use]
    pub fn as_str(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.as_bytes())
    }

    /// Returns the major version number.
    #[must_use]
    pub const fn major(&self) -> u8 {
        match self {
            Self::ZeroDotNine => 0,
            Self::OneDotZero | Self::OneDotOne => 1,
            Self::TwoDotZero => 2,
            Self::ThreeDotZero => 3,
        }
    }

    /// Returns the minor version number.
    #[must_use]
    pub const fn minor(&self) -> u8 {
        match self {
            Self::OneDotZero | Self::TwoDotZero | Self::ThreeDotZero => 0,
            Self::OneDotOne => 1,
            Self::ZeroDotNine => 9,
        }
    }

    /// Returns whether the protocol version is supported.
    #[must_use]
    pub fn is_supported(&self) -> bool {
        *self == Self::OneDotOne
    }
}
