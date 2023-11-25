use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use std::result::Result as StdResult;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseErrorKind {
    Uri,
    Method,
    Version,
    Status,
    RequestLine,
    StatusLine,
    Header,
    NonUtf8Header,
    Body,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Uri => f.write_str("URI parsing failed"),
            Self::Method => f.write_str("HTTP method parsing failed"),
            Self::Version => f.write_str("HTTP version parsing failed"),
            Self::Status => f.write_str("HTTP status parsing failed"),
            Self::RequestLine => f.write_str("request line parsing failed"),
            Self::StatusLine => f.write_str("status line parsing failed"),
            Self::Header => f.write_str("header parsing failed"),
            Self::NonUtf8Header => f.write_str("header name is not UTF-8 encoded"),
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
        Self::ParseError(kind)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NetError {
    HttpsNotImplemented,
    ParseError(ParseErrorKind),
    ReadError(IoErrorKind),
    WriteError(IoErrorKind),
    IoError(IoErrorKind),
}

impl StdError for NetError {}

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::HttpsNotImplemented => f.write_str("HTTPS is not implemented"),
            Self::ParseError(kind) => write!(f, "{kind}"),
            Self::ReadError(kind) => write!(f, "IO read error: {}", IoError::from(*kind)),
            Self::WriteError(kind) => write!(f, "IO write error: {}", IoError::from(*kind)),
            Self::IoError(kind) => write!(f, "IO error: {}", IoError::from(*kind)),
        }
    }
}

impl From<IoError> for NetError {
    #[allow(clippy::match_same_arms)]
    fn from(err: IoError) -> Self {
        match err.kind() {
            kind @ IoErrorKind::UnexpectedEof => Self::ReadError(kind),
            kind @ IoErrorKind::WouldBlock => Self::ReadError(kind),
            kind @ IoErrorKind::WriteZero => Self::WriteError(kind),
            kind => Self::IoError(kind),
        }
    }
}

impl From<NetError> for IoError {
    #[allow(clippy::match_same_arms)]
    fn from(err: NetError) -> Self {
        match err {
            NetError::HttpsNotImplemented => {
                Self::new(IoErrorKind::Unsupported, err.to_string())
            },
            NetError::ParseError(_) => err.into(),
            NetError::ReadError(kind) => Self::from(kind),
            NetError::WriteError(kind) => Self::from(kind),
            NetError::IoError(kind) => Self::from(kind),
        }
    }
}

pub type NetResult<T> = StdResult<T, NetError>;
