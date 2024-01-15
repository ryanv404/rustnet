use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::num::NonZeroU16;
use std::str::{self, FromStr};

use crate::NetParseError;

/// The HTTP method.
#[derive(Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum Method {
    /// Wildcard variant which represents any method value.
    Any,
    /// Transfers a current representation of the target resource.
    Get,
    /// Performs processing on the target resource.
    Post,
    /// Replaces all current representations of the target resource.
    Put,
    /// Performs a similar action to PUT but can do partial updates.
    Patch,
    /// Removes all current representations of the target resource.
    Delete,
    /// Performs the same action as GET but the response body is excluded.
    Head,
    /// Performs a message loop-back test along the target resource path.
    Trace,
    /// Describes the communication options for the target resource.
    Options,
    /// Establishes a tunnel to the server identified by the target resource.
    Connect,
    /// Used to gracefully shut down a test server.
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

impl Debug for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Method::{}", self.as_str())
    }
}

impl FromStr for Method {
    type Err = NetParseError;

    fn from_str(method: &str) -> Result<Self, Self::Err> {
        match method {
            // HTTP methods are case-sensitive.
            "ANY" => Ok(Self::Any),
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
    /// Returns the `Method` as a string slice.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Any => "ANY",
            Self::Get => "GET",
            Self::Put => "PUT",
            Self::Post => "POST",
            Self::Head => "HEAD",
            Self::Patch => "PATCH",
            Self::Trace => "TRACE",
            Self::Delete => "DELETE",
            Self::Options => "OPTIONS",
            Self::Connect => "CONNECT",
            Self::Shutdown => "SHUTDOWN",
        }
    }

    /// Returns the `Method` as a bytes slice.
    #[must_use]
    pub fn as_bytes(&self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    /// Returns true if this `Method` is not expected to cause a change in
    /// state on the server and is "essentially read-only".
    #[must_use]
    pub const fn is_safe(&self) -> bool {
        matches!(self, Self::Get | Self::Head | Self::Trace | Self::Options)
    }

    /// Returns true if multiple requests with this `Method` is expected to
    /// have the exact same effect on the server as a single request would.
    ///
    /// This is useful, for instance, when one wants to automatically retry
    /// a request even though a server response has not been received (such as
    /// when a connection closes unexpectedly).
    #[must_use]
    pub const fn is_idempotent(&self) -> bool {
        matches!(self, Self::Put | Self::Delete) || self.is_safe()
    }
}

/// The HTTP response status.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Status(pub NonZeroU16);

impl Default for Status {
    fn default() -> Self {
        // SAFETY: we know that `NonZeroU16::new` returns a `Some` variant
        // since we supplied the input.
        Self(NonZeroU16::new(200u16).unwrap())
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Status {
    type Err = NetParseError;

    fn from_str(status_code: &str) -> Result<Self, Self::Err> {
        u16::from_str(status_code)
            .map_err(|_| NetParseError::Status)
            .and_then(Self::try_from)
    }
}

impl TryFrom<&[u8]> for Status {
    type Error = NetParseError;

    fn try_from(status_code: &[u8]) -> Result<Self, Self::Error> {
        str::from_utf8(status_code)
            .map_err(|_| NetParseError::Status)
            .and_then(Self::from_str)
    }
}

impl TryFrom<u16> for Status {
    type Error = NetParseError;

    fn try_from(code: u16) -> Result<Self, Self::Error> {
        if !matches!(code, 100..=999) {
            return Err(NetParseError::Status);
        }

        NonZeroU16::new(code).map(Self).ok_or(NetParseError::Status)
    }
}

macro_rules! impl_status_methods {
    ($( $num:literal, $text:literal, $bytes:literal; )+) => {
        impl Status {
            /// Returns the `Status` as a copy-on-write string slice.
            #[must_use]
            pub fn as_str(&self) -> Cow<'static, str> {
                match self.code() {
                    $( $num => $text.into(), )+
                    code => format!("{code}").into(),
                }
            }

            /// Returns the `Status` as a copy-on-write bytes slice.
            #[must_use]
            pub fn as_bytes(&self) -> Cow<'static, [u8]> {
                match self.code() {
                    $( $num => $bytes[..].into(), )+
                    code => format!("{code}").into_bytes().into(),
                }
            }

            /// Returns a reason phrase for this `Status`, if possible.
            #[must_use]
            pub const fn msg(&self) -> Option<&'static str> {
                match self.code() {
                    $( $num => Some($text), )+
                    _ => None,
                }
            }

            /// Returns the status code as a u16 integer.
            #[must_use]
            pub const fn code(&self) -> u16 {
                self.0.get()
            }

            /// Returns true if the status code is greater than or equal to 100 and
            /// less than 200.
            #[must_use]
            pub const fn is_informational(&self) -> bool {
                matches!(self.code(), 100..=199)
            }

            /// Returns true if the status code is greater than or equal to 200 and
            /// less than 300.
            #[must_use]
            pub const fn is_success(&self) -> bool {
                matches!(self.code(), 200..=299)
            }

            /// Returns true if the status code is greater than or equal to 300 and
            /// less than 400.
            #[must_use]
            pub const fn is_redirection(&self) -> bool {
                matches!(self.code(), 300..=399)
            }

            /// Returns true if the status code is greater than or equal to 400 and
            /// less than 500.
            #[must_use]
            pub const fn is_client_error(&self) -> bool {
                matches!(self.code(), 400..=499)
            }

            /// Returns true if the status code is greater than or equal to 500 and
            /// less than 600.
            #[must_use]
            pub const fn is_server_error(&self) -> bool {
                matches!(self.code(), 500..=599)
            }
        }
    };
}

impl_status_methods! {
    100, "100 Continue",
        b"100 Continue";
    101, "101 Switching Protocols",
        b"101 Switching Protocols";
    102, "102 Processing",
        b"102 Processing";
    103, "103 Early Hints",
        b"103 Early Hints";
    200, "200 OK",
        b"200 OK";
    201, "201 Created",
        b"201 Created";
    202, "202 Accepted",
        b"202 Accepted";
    203, "203 Non-Authoritative Information",
        b"203 Non-Authoritative Information";
    204, "204 No Content",
        b"204 No Content";
    205, "205 Reset Content",
        b"205 Reset Content";
    206, "206 Partial Content",
        b"206 Partial Content";
    207, "207 Multi-Status",
        b"207 Multi-Status";
    208, "208 Already Reported",
        b"208 Already Reported";
    218, "218 This Is Fine",
        b"218 This Is Fine";
    226, "226 IM Used",
        b"226 IM Used";
    300, "300 Multiple Choices",
        b"300 Multiple Choices";
    301, "301 Moved Permanently",
        b"301 Moved Permanently";
    302, "302 Found",
        b"302 Found";
    303, "303 See Other",
        b"303 See Other";
    304, "304 Not Modified",
        b"304 Not Modified";
    305, "305 Use Proxy",
        b"305 Use Proxy";
    306, "306 Switch Proxy",
        b"306 Switch Proxy";
    307, "307 Temporary Redirect",
        b"307 Temporary Redirect";
    308, "308 Permanent Redirect",
        b"308 Permanent Redirect";
    400, "400 Bad Request",
        b"400 Bad Request";
    401, "401 Unauthorized",
        b"401 Unauthorized";
    402, "402 Payment Required",
        b"402 Payment Required";
    403, "403 Forbidden",
        b"403 Forbidden";
    404, "404 Not Found",
        b"404 Not Found";
    405, "405 Method Not Allowed",
        b"405 Method Not Allowed";
    406, "406 Not Acceptable",
        b"406 Not Acceptable";
    407, "407 Proxy Authentication Required",
        b"407 Proxy Authentication Required";
    408, "408 Request Timeout",
        b"408 Request Timeout";
    409, "409 Conflict",
        b"409 Conflict";
    410, "410 Gone",
        b"410 Gone";
    411, "411 Length Required",
        b"411 Length Required";
    412, "412 Precondition Failed",
        b"412 Precondition Failed";
    413, "413 Payload Too Large",
        b"413 Payload Too Large";
    414, "414 URI Too Long",
        b"414 URI Too Long";
    415, "415 Unsupported Media Type",
        b"415 Unsupported Media Type";
    416, "416 Range Not Satisfiable",
        b"416 Range Not Satisfiable";
    417, "417 Expectation Failed",
        b"417 Expectation Failed";
    418, "418 I'm a Teapot",
        b"418 I'm a Teapot";
    419, "419 Page Expired",
        b"419 Page Expired";
    420, "420 Method Failure or Enhance Your Calm",
        b"420 Method Failure or Enhance Your Calm";
    421, "421 Misdirected Request",
        b"421 Misdirected Request";
    422, "422 Unprocessable Entity",
        b"422 Unprocessable Entity";
    423, "423 Locked",
        b"423 Locked";
    424, "424 Failed Dependency",
        b"424 Failed Dependency";
    425, "425 Too Early",
        b"425 Too Early";
    426, "426 Upgrade Required",
        b"426 Upgrade Required";
    428, "428 Precondition Required",
        b"428 Precondition Required";
    429, "429 Too Many Requests",
        b"429 Too Many Requests";
    430, "430 HTTP Status Code",
        b"430 HTTP Status Code";
    431, "431 Request Header Fields Too Large",
        b"431 Request Header Fields Too Large";
    440, "440 Login Time-Out",
        b"440 Login Time-Out";
    444, "444 No Response",
        b"444 No Response";
    449, "449 Retry With",
        b"449 Retry With";
    450, "450 Blocked by Windows Parental Controls",
        b"450 Blocked by Windows Parental Controls";
    451, "451 Unavailable For Legal Reasons",
        b"451 Unavailable For Legal Reasons";
    460, "460 Client Closed Connection Prematurely",
        b"460 Client Closed Connection Prematurely";
    463, "463 Too Many Forwarded IP Addresses",
        b"463 Too Many Forwarded IP Addresses";
    464, "464 Incompatible Protocol",
        b"464 Incompatible Protocol";
    494, "494 Request Header Too Large",
        b"494 Request Header Too Large";
    495, "495 SSL Certificate Error",
        b"495 SSL Certificate Error";
    496, "496 SSL Certificate Required",
        b"496 SSL Certificate Required";
    497, "497 HTTP Request Sent to HTTPS Port",
        b"497 HTTP Request Sent to HTTPS Port";
    498, "498 Invalid Token",
        b"498 Invalid Token";
    499, "499 Token Required or Client Closed Request",
        b"499 Token Required or Client Closed Request";
    500, "500 Internal Server Error",
        b"500 Internal Server Error";
    501, "501 Not Implemented",
        b"501 Not Implemented";
    502, "502 Bad Gateway",
        b"502 Bad Gateway";
    503, "503 Service Unavailable",
        b"503 Service Unavailable";
    504, "504 Gateway Timeout",
        b"504 Gateway Timeout";
    505, "505 HTTP Version Not Supported",
        b"505 HTTP Version Not Supported";
    506, "506 Variant Also Negotiates",
        b"506 Variant Also Negotiates";
    507, "507 Insufficient Storage",
        b"507 Insufficient Storage";
    508, "508 Loop Detected",
        b"508 Loop Detected";
    509, "509 Bandwidth Limit Exceeded",
        b"509 Bandwidth Limit Exceeded";
    510, "510 Not Extended",
        b"510 Not Extended";
    511, "511 Network Authentication Required",
        b"511 Network Authentication Required";
    520, "520 Web Server Is Returning an Unknown Error",
        b"520 Web Server Is Returning an Unknown Error";
    521, "521 Web Server Is Down",
        b"521 Web Server Is Down";
    522, "522 Connection Timed Out",
        b"522 Connection Timed Out";
    523, "523 Origin Is Unreachable",
        b"523 Origin Is Unreachable";
    524, "524 A Timeout Occurred",
        b"524 A Timeout Occurred";
    525, "525 SSL Handshake Failed",
        b"525 SSL Handshake Failed";
    526, "526 Invalid SSL Certificate",
        b"526 Invalid SSL Certificate";
    527, "527 Railgun Listener to Origin",
        b"527 Railgun Listener to Origin";
    529, "529 The Service Is Overloaded",
        b"529 The Service Is Overloaded";
    530, "530 Site Frozen",
        b"530 Site Frozen";
    561, "561 Unauthorized",
        b"561 Unauthorized";
    598, "598 Network Read Timeout Error",
        b"598 Network Read Timeout Error";
    599, "599 Network Connect Timeout Error",
        b"599 Network Connect Timeout Error";
    999, "999 Request Denied",
        b"999 Request Denied";
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
            // A trailing ".0" is implied if the decimal is missing.
            "HTTP/2" | "HTTP/2.0" => Ok(Self::TwoDotZero),
            "HTTP/3" | "HTTP/3.0" => Ok(Self::ThreeDotZero),
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
    /// Returns the the protocol `Version` as a string slice.
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

    /// Returns the the protocol `Version` as a bytes slice.
    #[must_use]
    pub fn as_bytes(&self) -> &'static [u8] {
        self.as_str().as_bytes()
    }

    /// Returns the protocol's major version number.
    #[must_use]
    pub const fn major(&self) -> u8 {
        match self {
            Self::ZeroDotNine => 0,
            Self::OneDotZero | Self::OneDotOne => 1,
            Self::TwoDotZero => 2,
            Self::ThreeDotZero => 3,
        }
    }

    /// Returns the protocol's minor version number.
    #[must_use]
    pub const fn minor(&self) -> u8 {
        match self {
            Self::OneDotZero | Self::TwoDotZero | Self::ThreeDotZero => 0,
            Self::OneDotOne => 1,
            Self::ZeroDotNine => 9,
        }
    }

    /// Returns true if the protocol version is currently supported.
    #[must_use]
    pub fn is_supported(&self) -> bool {
        matches!(self, Self::OneDotOne)
    }
}
