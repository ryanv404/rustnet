use std::{error, fmt, io::{self, ErrorKind}};

#[derive(Clone, Debug)]
pub enum NetError {
    BadBufferRead,
    BadRequest,
    BadRequestLine,
    BadRequestHeader,
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadBufferRead => f.write_str("network reader error"),
            Self::BadRequest => f.write_str("invalid request"),
            Self::BadRequestLine => f.write_str("invalid request line"),
            Self::BadRequestHeader => f.write_str("invalid request header"),
        }
    }
}

impl error::Error for NetError {}

impl From<NetError> for io::Error {
    fn from(err: NetError) -> Self {
        match err {
            NetError::BadBufferRead => Self::new(
                ErrorKind::InvalidData, "unable to read from the network reader"
            ),
            NetError::BadRequest => Self::new(
                ErrorKind::InvalidData, "unable to parse the request"
            ),
            NetError::BadRequestLine => Self::new(
                ErrorKind::InvalidData, "unable to parse the request line"
            ),
            NetError::BadRequestHeader => Self::new(
                ErrorKind::InvalidData, "unable to parse a request header"
            ),
        }
    }
}
