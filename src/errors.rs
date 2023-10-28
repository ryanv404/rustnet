use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    io::Error as IoError,
    result::Result as StdResult,
};

pub type NetResult<T> = StdResult<T, NetError>;

#[derive(Debug)]
pub enum NetError {
    EarlyEof,
    NonUtf8Header,
    BadStatusCode,
    IoError(IoError),
    ParseError(&'static str),
    BadConnection(IoError),
}

impl Error for NetError {}

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::EarlyEof => f.write_str("received an unexpected EOF on the stream"),
            Self::NonUtf8Header => f.write_str("header names must only contain UTF-8 bytes"),
            Self::BadStatusCode => f.write_str("invalid status code"),
            Self::IoError(e) => write!(f, "read error: {e}"),
            Self::ParseError(s) => write!(f, "unable to parse: {s}"),
            Self::BadConnection(e) => write!(f, "connection error: {e}"),
        }
    }
}

impl From<IoError> for NetError {
    fn from(e: IoError) -> Self {
        Self::IoError(e)
    }
}
