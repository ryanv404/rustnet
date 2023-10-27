use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
    io::Error as IoError,
    result::Result as StdResult,
};

pub type NetResult<T> = StdResult<T, NetError>;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum NetError {
    BadStatus,
    EarlyEof,
    ReadError(IoError),
    ParseError(&'static str),
    BadConnection(IoError),
}

impl Error for NetError {}

impl Display for NetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::BadStatus => f.write_str("invalid status code"),
            Self::EarlyEof => {
                f.write_str("received an unexpected EOF on the stream")
            },
            Self::ReadError(e) => write!(f, "read error: {e}"),
            Self::ParseError(s) => write!(f, "unable to parse: {s}"),
            Self::BadConnection(e) => write!(f, "connection error: {e}"),
        }
    }
}

impl From<IoError> for NetError {
    fn from(e: IoError) -> Self {
        Self::ReadError(e)
    }
}
