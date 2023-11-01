use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{Error as IoError, ErrorKind as IoErrorKind};

#[derive(Debug)]
pub enum NetError {
    NonUtf8Header,
    BadStatusCode,
    IoError(IoError),
    ReadError(IoError),
    WriteError(IoError),
    ParseError(&'static str),
}

impl Error for NetError {}

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::NonUtf8Header => f.write_str("non-UTF-8 encoded header name"),
            Self::BadStatusCode => f.write_str("invalid status code"),
            Self::IoError(e) => write!(f, "io error: {e}"),
            Self::ReadError(e) => write!(f, "read error: {e}"),
            Self::WriteError(e) => write!(f, "write error: {e}"),
            Self::ParseError(s) => write!(f, "unable to parse: {s}"),
        }
    }
}

impl From<IoError> for NetError {
    fn from(e: IoError) -> Self {
        Self::IoError(e)
    }
}

impl NetError {
    #[must_use]
    pub fn from_kind(kind: IoErrorKind) -> Self {
        Self::IoError(IoError::from(kind))
    }
}
