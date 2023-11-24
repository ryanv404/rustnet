//! `rustnet`
//! A Rust library for building an HTTP server.

#![deny(clippy::cargo)]
#![deny(clippy::complexity)]
#![deny(clippy::correctness)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![deny(clippy::suspicious)]
#![allow(clippy::similar_names)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]

use std::collections::{BTreeMap, BTreeSet};
use std::result::Result as StdResult;

#[cfg(test)]
mod tests;

pub mod client;
pub mod connection;
pub mod errors;
pub mod header;
pub mod http;
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod threadpool;
pub mod util;

pub use client::Client;
pub use connection::{Connection, NetReader, NetWriter};
pub use errors::{NetError, ParseErrorKind};
pub use header::{Header, HeaderName, HeaderValue};
pub use http::{Method, Status, Version};
pub use request::{Request, RequestBuilder, RequestLine};
pub use response::{Response, ResponseBuilder, StatusLine};
pub use router::{Resolved, Route, Router, Target};
pub use server::{Server, ServerConfig};
pub use threadpool::{ThreadPool, Worker};
pub use util::{trim_whitespace_bytes, get_datetime};

pub mod consts {
    pub use crate::header::header_consts::*;
    pub const MAX_HEADERS: u16 = 1024;
    pub const NUM_WORKER_THREADS: usize = 4;
    pub const READER_BUFSIZE: usize = 1024;
    pub const WRITER_BUFSIZE: usize = 1024;
    pub const DEFAULT_NAME: &str = concat!("rustnet/", env!("CARGO_PKG_VERSION"));
}

pub type RoutesMap = BTreeMap<Route, Target>;
pub type HeadersSet = BTreeSet<Header>;
pub type NetResult<T> = StdResult<T, NetError>;
