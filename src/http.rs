use std::{fmt, io::{self, ErrorKind}};

/// HTTP methods.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Method {
    Get,
    Put,
    Post,
    Head,
    Patch,
    Trace,
    Delete,
    Connect,
    Options,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&[u8]> for Method {
    type Error = io::Error;

    fn try_from(bytes: &[u8]) -> io::Result<Self> {
        let method = match bytes {
            b"GET" => Self::Get,
            b"PUT" => Self::Put,
            b"POST" => Self::Post,
            b"HEAD" => Self::Head,
            b"PATCH" => Self::Patch,
            b"TRACE" => Self::Trace,
            b"DELETE" => Self::Delete,
            b"CONNECT" => Self::Connect,
            b"OPTIONS" => Self::Options,
            unk => {
                let unknown = String::from_utf8_lossy(unk);
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Unsupported HTTP method \"{unknown}\"requested.")
                ));
            },
        };
        Ok(method)
    }
}

impl Method {
    pub fn as_str(&self) -> &'static str {
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

/// HTTP status codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Status(pub u16);

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.code(), self.msg())
    }
}

impl Status {
    #[must_use]
    pub const fn msg(&self) -> &'static str {
        match self.0 {
            // 1xx (informational) status codes.
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            103 => "Early Hints",

            // 2xx (successful) status codes.
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

            // 3xx (redirect) status codes.
            300 => "Multiple Choices",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            305 => "Use Proxy",
            306 => "Switch Proxy",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",

            // 4xx (client error) status codes.
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
            414 => "URI Too Long",
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

            // 5xx (server error) status codes.
            500 => "Internal Server Error",
            501 => "Not Implemented",
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
            _ => ""
        }
    }

    #[must_use]
    pub const fn code(&self) -> u16 {
        self.0
    }
}

// The HTTP protocol version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Version {
    ZeroDotNine,
    OneDotZero,
    OneDotOne,
    TwoDotZero,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&[u8]> for Version {
    type Error = io::Error;

    fn try_from(bytes: &[u8]) -> io::Result<Self> {
        let version = match bytes {
            b"HTTP/0.9" => Self::ZeroDotNine,
            b"HTTP/1.0" => Self::OneDotZero,
            b"HTTP/1.1" => Self::OneDotOne,
            b"HTTP/2.0" => Self::TwoDotZero,
            unk => {
                let unknown = String::from_utf8_lossy(unk);
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Unsupported HTTP version \"{unknown}\"requested.")
                ));
            },
        };
        Ok(version)
    }
}

impl Version {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ZeroDotNine => "HTTP/0.9",
            Self::OneDotZero => "HTTP/1.0",
            Self::OneDotOne => "HTTP/1.1",
            Self::TwoDotZero => "HTTP/2.0",
        }
    }
}
