//! # rustnet
//!
//! An HTTP networking library for building servers and clients.

#![deny(clippy::all)]
#![deny(clippy::cargo)]
#![deny(clippy::complexity)]
#![deny(clippy::correctness)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![deny(clippy::perf)]
#![deny(clippy::style)]
#![deny(clippy::suspicious)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]

#[cfg(test)]
mod tests;

pub mod body;
pub mod cli;
pub mod client;
pub mod errors;
pub mod header;
pub mod http;
pub mod io;
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod thread_pool;
pub mod util;

pub use body::Body;
pub use cli::{ClientCli, ServerCli};
pub use client::{Client, ClientBuilder, OutputStyle, Tui, WriteCliError};
pub use errors::{NetError, NetParseError, NetResult};
pub use header::{
    Header, HeaderName, HeaderNameInner, HeaderValue, Headers, header_name,
};
pub use io::Connection;
pub use http::{Method, Status, StatusCode, Version};
pub use request::{Request, RequestLine};
pub use response::{Response, StatusLine};
pub use router::{Route, RouteBuilder, Router, Target};
pub use server::{
    Server, ServerBuilder, ServerConfig, ServerHandle,
};
pub use thread_pool::{ThreadPool, Worker};

pub mod colors {
    pub const RED: &str = "\x1b[91m";
    pub const GRN: &str = "\x1b[92m";
    pub const YLW: &str = "\x1b[93m";
    pub const BLU: &str = "\x1b[94m";
    pub const PURP: &str = "\x1b[95m";
    pub const CYAN: &str = "\x1b[96m";
    pub const CLR: &str = "\x1b[0m";
}

pub const NUM_WORKERS: usize = 4;
pub const READER_BUFSIZE: usize = 1024;
pub const WRITER_BUFSIZE: usize = 1024;
