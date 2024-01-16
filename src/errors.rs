use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::result::Result;

/// Result type that contains a `NetError` error variant.
pub type NetResult<T> = Result<T, NetError>;

/// Errors representing the various potential points of failure.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetError {
    BadAddress,
    BadBody,
    BadHeader,
    BadHeaderName,
    BadHeaderValue,
    BadMethod,
    BadPath,
    BadRequest,
    BadResponse,
    BadScheme,
    BadStatusCode,
    BadUri,
    BadVersion,
    HttpsNotImplemented,
    IoError(IoErrorKind),
    JoinFailure,
    NotConnected,
    NoRequest,
    NoResponse,
    Other(Cow<'static, str>),
    Read(IoErrorKind),
    TooManyHeaders,
    UnexpectedEof,
    Write(IoErrorKind),
}

impl Error for NetError {}

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::BadAddress => f.write_str("Address parsing failed"),
            Self::BadBody => f.write_str("Body parsing failed"),
            Self::BadHeader => f.write_str("Header parsing failed"),
            Self::BadHeaderName => f.write_str("Header name parsing failed"),
            Self::BadHeaderValue => f.write_str("Header value parsing failed"),
            Self::BadMethod => f.write_str("Method parsing failed"),
            Self::BadPath => f.write_str("URI path parsing failed"),
            Self::BadRequest => f.write_str("Request parsing failed"),
            Self::BadResponse => f.write_str("Response parsing failed"),
            Self::BadScheme => f.write_str("URI scheme parsing failed"),
            Self::BadStatusCode => f.write_str("Status code parsing failed"),
            Self::BadUri => f.write_str("URI parsing failed"),
            Self::BadVersion => f.write_str("Version parsing failed"),
            Self::HttpsNotImplemented => f.write_str("HTTPS not implemented"),
            Self::IoError(kind) => write!(f, "Received \"{kind}\" error"),
            Self::JoinFailure => f.write_str("Could not join server thread"),
            Self::NotConnected => f.write_str("No active TCP connection"),
            Self::NoRequest => f.write_str("No request found"),
            Self::NoResponse => f.write_str("No response found"),
            Self::Other(ref err_msg) => write!(f, "{err_msg}"),
            Self::Read(kind) => write!(f, "Received \"{kind}\" read error"),
            Self::TooManyHeaders => f.write_str("Too many headers"),
            Self::UnexpectedEof => f.write_str("Received unexpected EOF"),
            Self::Write(kind) => write!(f, "Received \"{kind}\" write error"),
        }
    }
}

impl From<IoErrorKind> for NetError {
    fn from(kind: IoErrorKind) -> Self {
        match kind {
            IoErrorKind::NotConnected => Self::NotConnected,
            IoErrorKind::UnexpectedEof => Self::UnexpectedEof,
            _ => Self::IoError(kind),
        }
    }
}

impl From<IoError> for NetError {
    fn from(err: IoError) -> Self {
        err.kind().into()
    }
}

impl From<NetError> for IoError {
    fn from(err: NetError) -> Self {
        match err {
            NetError::IoError(kind)
                | NetError::Read(kind)
                | NetError::Write(kind) =>
            {
                Self::from(kind)
            },
            NetError::HttpsNotImplemented => {
                Self::new(IoErrorKind::Unsupported, err)
            },
            NetError::NotConnected => {
                Self::from(IoErrorKind::NotConnected)
            },
            NetError::UnexpectedEof => {
                Self::from(IoErrorKind::UnexpectedEof)
            },
            NetError::BadAddress
                | NetError::BadBody
                | NetError::BadHeader
                | NetError::BadHeaderName
                | NetError::BadHeaderValue
                | NetError::BadMethod
                | NetError::BadPath
                | NetError::BadRequest
                | NetError::BadResponse
                | NetError::BadScheme
                | NetError::BadStatusCode
                | NetError::BadUri
                | NetError::BadVersion
                | NetError::JoinFailure
                | NetError::NoRequest
                | NetError::NoResponse
                | NetError::TooManyHeaders =>
            {
                Self::new(IoErrorKind::Other, err)
            },
            NetError::Other(msg) => Self::new(IoErrorKind::Other, msg),
        }
    }
}
