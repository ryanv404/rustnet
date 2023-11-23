use std::error::Error as StdError;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseErrorKind {
    Uri,
    Method,
    Version,
    Status,
    NonUtf8Header,
    ReqLine,
    Header,
    ReqBody,
    ResBody,
    Request,
    Response,
}

impl Display for ParseErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Uri => f.write_str("URI parsing failed"),
            Self::Method => f.write_str("HTTP method parsing failed"),
            Self::Version => f.write_str("HTTP version parsing failed"),
            Self::Status => f.write_str("HTTP status parsing failed"),
            Self::ReqLine => f.write_str("request line parsing failed"),
            Self::Header => f.write_str("header parsing failed"),
            Self::ReqBody => f.write_str("request body parsing failed"),
            Self::ResBody => f.write_str("response body parsing failed"),
            Self::Request => f.write_str("request parsing failed"),
            Self::Response => f.write_str("response parsing failed"),
            Self::NonUtf8Header => f.write_str("header name is not UTF-8 encoded"),
        }
    }
}

impl From<ParseErrorKind> for IoError {
    fn from(kind: ParseErrorKind) -> Self {
        IoError::new(IoErrorKind::InvalidData, kind.to_string())
    }
}

impl From<ParseErrorKind> for NetError {
    fn from(kind: ParseErrorKind) -> Self {
        NetError::ParseError(kind)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetError {
    UnexpectedEof,
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
            Self::UnexpectedEof => f.write_str("Read an unexpected EOF"),
            Self::HttpsNotImplemented => f.write_str("HTTPS is not implemented"),
            Self::ParseError(kind) => write!(f, "{kind}"),
            Self::ReadError(kind) => write!(f, "IO read error: {}", IoError::from(*kind)),
            Self::WriteError(kind) => write!(f, "IO write error: {}", IoError::from(*kind)),
            Self::IoError(kind) => write!(f, "IO error: {}", IoError::from(*kind)),
        }
    }
}

impl From<IoError> for NetError {
    fn from(err: IoError) -> Self {
        let kind = err.kind();
        match kind {
            IoErrorKind::UnexpectedEof => Self::UnexpectedEof,
            IoErrorKind::WouldBlock => Self::ReadError(kind),
            IoErrorKind::WriteZero => Self::WriteError(kind),
            _ => Self::IoError(kind),
        }
    }
}

impl From<NetError> for IoError {
    fn from(err: NetError) -> Self {
        match err {
            NetError::UnexpectedEof => {
                IoError::from(IoErrorKind::UnexpectedEof)
            },
            NetError::HttpsNotImplemented => {
                IoError::new(IoErrorKind::Unsupported, err.to_string())
            },
            NetError::ParseError(_) => err.into(),
            NetError::ReadError(kind) => IoError::from(kind),
            NetError::WriteError(kind) => IoError::from(kind),
            NetError::IoError(kind) => IoError::from(kind),
        }
    }
}
