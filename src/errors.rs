use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::result::Result as StdResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetParseError {
    Body,
    Header,
    Method,
    StatusCode,
    StatusLine,
    UriPath,
    Version,
}

impl StdError for NetParseError {}

impl Display for NetParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Could not parse the {}",
            match self {
                Self::Body => "message body",
                Self::Header => "headers",
                Self::Method => "HTTP method",
                Self::StatusCode => "status code",
                Self::StatusLine => "status line",
                Self::UriPath => "URI path",
                Self::Version => "HTTP version",
            }
        )
    }
}

impl From<NetParseError> for IoError {
    fn from(err: NetParseError) -> Self {
        Self::new(IoErrorKind::Other, err)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetError {
    Https,
    Io(IoErrorKind),
    NotConnected,
    Other(&'static str),
    Parse(NetParseError),
    Read(IoErrorKind),
    UnexpectedEof,
    Write(IoErrorKind),
}

impl StdError for NetError {}

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Https => f.write_str("HTTPS is not implemented"),
            Self::Io(kind) => write!(f, "I/O error: {kind}"),
            Self::NotConnected => f.write_str("No active TCP stream found"),
            Self::Other(msg) => write!(f, "Error: {msg}"),
            Self::Parse(kind) => write!(f, "Parsing error: {kind}"),
            Self::Read(kind) => write!(f, "Read error: {kind}"),
            Self::UnexpectedEof => f.write_str("Received an unexpected EOF"),
            Self::Write(kind) => write!(f, "Write error: {kind}"),
        }
    }
}

impl From<IoError> for NetError {
    fn from(err: IoError) -> Self {
        match err.kind() {
            IoErrorKind::UnexpectedEof => Self::UnexpectedEof,
            kind @ IoErrorKind::WriteZero => Self::Write(kind),
            kind => Self::Io(kind),
        }
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

impl From<NetParseError> for NetError {
    fn from(err: NetParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<NetError> for IoError {
    fn from(err: NetError) -> Self {
        match err {
            NetError::Https => Self::new(IoErrorKind::Unsupported, err),
            NetError::NotConnected
            | NetError::Other(_)
            | NetError::UnexpectedEof
            | NetError::Parse(_) => Self::new(IoErrorKind::Other, err),
            NetError::Read(kind)
            | NetError::Write(kind)
            | NetError::Io(kind) => Self::from(kind),
        }
    }
}

pub type NetResult<T> = StdResult<T, NetError>;
