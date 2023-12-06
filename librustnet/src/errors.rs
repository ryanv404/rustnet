use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::result::Result as StdResult;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseErrorKind {
    Path,
    Method,
    Version,
    Status,
    RequestLine,
    StatusLine,
    Header,
    Body,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Path => f.write_str("URI path parsing failed"),
            Self::Method => f.write_str("method parsing failed"),
            Self::Version => f.write_str("version parsing failed"),
            Self::Status => f.write_str("status parsing failed"),
            Self::RequestLine => f.write_str("request line parsing failed"),
            Self::StatusLine => f.write_str("status line parsing failed"),
            Self::Header => f.write_str("header parsing failed"),
            Self::Body => f.write_str("body parsing failed"),
        }
    }
}

impl From<ParseErrorKind> for IoError {
    fn from(kind: ParseErrorKind) -> Self {
        Self::new(IoErrorKind::InvalidData, kind.to_string())
    }
}

impl From<ParseErrorKind> for NetError {
    fn from(kind: ParseErrorKind) -> Self {
        Self::Parse(kind)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetError {
    HttpsNotImplemented,
    Parse(ParseErrorKind),
    Read(IoErrorKind),
    Write(IoErrorKind),
    Io(IoErrorKind),
}

impl StdError for NetError {}

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::HttpsNotImplemented => f.write_str("HTTPS is not implemented"),
            Self::Parse(kind) => write!(f, "{kind}"),
            Self::Read(kind) => write!(f, "IO read error: {}", IoError::from(*kind)),
            Self::Write(kind) => write!(f, "IO write error: {}", IoError::from(*kind)),
            Self::Io(kind) => write!(f, "IO error: {}", IoError::from(*kind)),
        }
    }
}

impl From<IoError> for NetError {
    fn from(err: IoError) -> Self {
        match err.kind() {
            kind @ IoErrorKind::UnexpectedEof => Self::Read(kind),
            kind @ IoErrorKind::WriteZero => Self::Write(kind),
            kind => Self::Io(kind),
        }
    }
}

impl From<IoErrorKind> for NetError {
    fn from(kind: IoErrorKind) -> Self {
        match kind {
            IoErrorKind::UnexpectedEof => Self::Read(kind),
            IoErrorKind::WriteZero => Self::Write(kind),
            kind => Self::Io(kind),
        }
    }
}

impl From<NetError> for IoError {
    fn from(err: NetError) -> Self {
        match err {
            NetError::HttpsNotImplemented => {
                Self::new(IoErrorKind::Unsupported, err.to_string())
            },
            NetError::Parse(_) => err.into(),
            NetError::Read(kind)
                | NetError::Write(kind) 
                | NetError::Io(kind) => Self::from(kind),
        }
    }
}

pub type NetResult<T> = StdResult<T, NetError>;
