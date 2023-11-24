use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

use crate::{NetError, NetResult, ParseErrorKind};

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
    /// Establishes a tunnel to the server identified by the target resource.
    Connect,
    /// Describes the communication options for the target resource.
    Options,
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
    type Err = NetError;

    fn from_str(s: &str) -> NetResult<Self> {
        match s {
            // HTTP methods are case-sensitive.
            "GET" => Ok(Self::Get),
            "PUT" => Ok(Self::Put),
            "POST" => Ok(Self::Post),
            "HEAD" => Ok(Self::Head),
            "PATCH" => Ok(Self::Patch),
            "TRACE" => Ok(Self::Trace),
            "DELETE" => Ok(Self::Delete),
            "CONNECT" => Ok(Self::Connect),
            "OPTIONS" => Ok(Self::Options),
            _ => Err(ParseErrorKind::Method.into()),
        }
    }
}

impl Method {
    /// Returns the HTTP method as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Put => "PUT",
            Self::Post => "POST",
            Self::Head => "HEAD",
            Self::Patch => "PATCH",
            Self::Trace => "TRACE",
            Self::Delete => "DELETE",
            Self::Connect => "CONNECT",
            Self::Options => "OPTIONS",
        }
    }
}

/// HTTP status code.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct Status(pub u16);

impl Default for Status {
    fn default() -> Self {
        Self(200)
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} {}", self.code(), self.msg())
    }
}

impl FromStr for Status {
    type Err = NetError;

    fn from_str(code: &str) -> NetResult<Self> {
        u16::from_str(code)
            .map(|num| Self::from(num))
            .map_err(|_| ParseErrorKind::Status.into())
    }
}

impl From<u16> for Status {
    fn from(code: u16) -> Self {
        Self(code)
    }
}

impl Status {
    /// Returns the reason phrase for a status.
    #[must_use]
    #[rustfmt::skip]
    #[allow(clippy::match_same_arms)]
    pub const fn msg(&self) -> &'static str {
        match self.0 {
            // 1xx (Informational) Statuses.
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            103 => "Early Hints",

            // 2xx (Successful) Statuses.
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            203 => "Non-Authoritative Information",
            204 => "No Content",
            205 => "Reset Content",
            206 => "Partial Content",
            207 => "Multi-Status",
            208 => "Already Reported",
            218 => "This Is Fine",
            226 => "IM Used",

            // 3xx (Redirect) Statuses.
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            306 => "Switch Proxy",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",

            // 4xx (Client Error) Statuses.
            400 => "Bad Request", // No or multiple Host headers, invalid request line.
            401 => "Unauthorized",
            402 => "Payment Required",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            406 => "Not Acceptable",
            407 => "Proxy Authentication Required",
            408 => "Request Timeout",
            409 => "Conflict",
            410 => "Gone",
            411 => "Length Required",
            412 => "Precondition Failed",
            413 => "Payload Too Large",
            414 => "URI Too Long", // Recommended to support 8kb+ request lines.
            415 => "Unsupported Media Type",
            416 => "Range Not Satisfiable",
            417 => "Expectation Failed",
            418 => "I'm a Teapot",
            419 => "Page Expired",
            420 => "Method Failure or Enhance Your Calm",
            421 => "Misdirected Request",
            422 => "Unprocessable Entity",
            423 => "Locked",
            424 => "Failed Dependency",
            425 => "Too Early",
            426 => "Upgrade Required",
            428 => "Precondition Required",
            429 => "Too Many Requests",
            430 => "HTTP Status Code",
            431 => "Request Header Fields Too Large",
            440 => "Login Time-Out",
            444 => "No Response",
            449 => "Retry With",
            450 => "Blocked by Windows Parental Controls",
            451 => "Unavailable For Legal Reasons",
            460 => "Client Closed Connection Prematurely",
            463 => "Too Many Forwarded IP Addresses",
            464 => "Incompatible Protocol",
            494 => "Request Header Too Large",
            495 => "SSL Certificate Error",
            496 => "SSL Certificate Required",
            497 => "HTTP Request Sent to HTTPS Port",
            498 => "Invalid Token",
            499 => "Token Required or Client Closed Request",

            // 5xx (Server Error) Statuses.
            500 => "Internal Server Error",
            501 => "Not Implemented", // Unimplemented methods, etc.
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            505 => "HTTP Version Not Supported",
            506 => "Variant Also Negotiates",
            507 => "Insufficient Storage",
            508 => "Loop Detected",
            509 => "Bandwidth Limit Exceeded",
            510 => "Not Extended",
            511 => "Network Authentication Required",
            520 => "Web Server Is Returning an Unknown Error",
            521 => "Web Server Is Down",
            522 => "Connection Timed Out",
            523 => "Origin Is Unreachable",
            524 => "A Timeout Occurred",
            525 => "SSL Handshake Failed",
            526 => "Invalid SSL Certificate",
            527 => "Railgun Listener to Origin",
            529 => "The Service Is Overloaded",
            530 => "Site Frozen",
            561 => "Unauthorized",
            598 => "Network Read Timeout Error",
            599 => "Network Connect Timeout Error",
            _ => "",
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
    type Err = NetError;

    fn from_str(s: &str) -> NetResult<Self> {
        // HTTP versions are case-sensitive.
        // Zero is implied by a missing minor version number.
        match s {
            "HTTP/0.9" => Ok(Self::ZeroDotNine),
            "HTTP/1.0" => Ok(Self::OneDotZero),
            "HTTP/1.1" => Ok(Self::OneDotOne),
            "HTTP/2" | "HTTP/2.0" => Ok(Self::TwoDotZero),
            "HTTP/3" | "HTTP/3.0" => Ok(Self::ThreeDotZero),
            _ => Err(ParseErrorKind::Version.into()),
        }
    }
}

impl Version {
    /// Returns the the protocol version as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ZeroDotNine => "HTTP/0.9",
            Self::OneDotZero => "HTTP/1.0",
            Self::OneDotOne => "HTTP/1.1",
            Self::TwoDotZero => "HTTP/2.0",
            Self::ThreeDotZero => "HTTP/3.0",
        }
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
    /// Currently only HTTP version 1.1 is supported.
    #[must_use]
    pub fn is_supported(&self) -> bool {
        *self == Self::OneDotOne
    }
}
