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

use std::result::Result as StdResult;

#[cfg(test)]
mod tests;

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

pub use connection::{NetReader, NetWriter, RemoteClient};
pub use errors::NetError;
pub use header::{HeaderName, HeaderValue, default_headers};
pub use http::{Method, Status, Version};
pub use request::Request;
pub use response::Response;
pub use router::{Resolved, Route, Router};
pub use server::{Server, ServerConfig};
pub use threadpool::{ThreadPool, Worker};
pub use util::{trim_whitespace_bytes, try_date};

pub mod consts {
    pub use crate::header::header_names::*;

	pub const NUM_WORKERS: usize = 4;
    pub const READER_BUFSIZE: usize = 1024;
    pub const WRITER_BUFSIZE: usize = 1024;
}

pub type NetResult<T> = StdResult<T, NetError>;
