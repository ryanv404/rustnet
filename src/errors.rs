use std::borrow::Cow;
use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::result::Result as StdResult;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetParseError {
    Body,
    Header,
    Method,
    Path,
    RequestLine,
    Status,
    StatusCode,
    StatusLine,
    TooManyHeaders,
    Version,
}

impl StdError for NetParseError {}

impl Display for NetParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Parsing error: {}",
            match self {
                Self::Body => "body",
                Self::Header => "headers",
                Self::Method => "method",
                Self::RequestLine => "request line",
                Self::Status => "status",
                Self::StatusCode => "status code",
                Self::StatusLine => "status line",
                Self::TooManyHeaders => "headers (exceeded max)",
                Self::Path => "URI path",
                Self::Version => "version",
            }
        )
    }
}

impl Debug for NetParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self}")
    }
}

impl From<NetParseError> for IoError {
    fn from(err: NetParseError) -> Self {
        Self::new(IoErrorKind::Other, err)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetError {
    ConnectFailure,
    JoinFailure,
    Https,
    Io(IoErrorKind),
    NoRequest,
    NoResponse,
    NotConnected,
    Other(Cow<'static, str>),
    Parse(NetParseError),
    Read(IoErrorKind),
    UnexpectedEof,
    Write(IoErrorKind),
}

impl StdError for NetError {}

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::ConnectFailure => f.write_str("Could not connect."),
            Self::JoinFailure => f.write_str("Could not join server thread."),
            Self::Https => f.write_str("HTTPS not implemented"),
            Self::Io(kind) => write!(f, "I/O error: {kind}"),
            Self::NoRequest => f.write_str("No request found."),
            Self::NoResponse => f.write_str("No response found."),
            Self::NotConnected => f.write_str("No active TCP stream"),
            Self::Other(ref msg) => write!(f, "{msg}"),
            Self::Parse(kind) => write!(f, "{kind}"),
            Self::Read(kind) => write!(f, "Read error: {kind}"),
            Self::UnexpectedEof => f.write_str("Received unexpected EOF"),
            Self::Write(kind) => write!(f, "Write error: {kind}"),
        }
    }
}

impl Debug for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self}")
    }
}

impl From<IoErrorKind> for NetError {
    fn from(kind: IoErrorKind) -> Self {
        match kind {
            IoErrorKind::UnexpectedEof => Self::UnexpectedEof,
            kind @ IoErrorKind::WriteZero => Self::Write(kind),
            kind => Self::Io(kind),
        }
    }
}

impl From<IoError> for NetError {
    fn from(err: IoError) -> Self {
        err.kind().into()
    }
}

impl From<NetParseError> for NetError {
    fn from(err: NetParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<NetError> for IoError {
    fn from(err: NetError) -> Self {
        match err {
            NetError::Https => Self::new(IoErrorKind::Unsupported, err),
            NetError::NotConnected | NetError::ConnectFailure => {
                Self::from(IoErrorKind::NotConnected)
            },
            NetError::UnexpectedEof => {
                Self::from(IoErrorKind::UnexpectedEof)
            },
            NetError::JoinFailure
                | NetError::NoRequest
                | NetError::NoResponse
                | NetError::Other(_)
                | NetError::Parse(_) => Self::new(IoErrorKind::Other, err),
            NetError::Read(kind)
                | NetError::Write(kind)
                | NetError::Io(kind) => Self::from(kind),
        }
    }
}

pub type NetResult<T> = StdResult<T, NetError>;
