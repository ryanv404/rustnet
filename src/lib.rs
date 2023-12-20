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
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod util;

pub use body::Body;
pub use cli::{ClientCli, ServerCli};
pub use client::{Client, ClientBuilder};
pub use errors::{NetError, NetParseError, NetResult};
pub use header::{Header, HeaderKind, HeaderName, HeaderValue, Headers};
pub use http::{Method, Status, Version};
pub use request::{NetReader, Request, RequestLine};
pub use response::{NetWriter, Response, StatusLine};
pub use router::{Route, RouteBuilder, Router, Target};
pub use server::{
    Connection, Server, ServerBuilder, ServerConfig, ServerHandle, ThreadPool,
    Worker,
};

pub const MAX_HEADERS: u16 = 1024;
pub const READER_BUFSIZE: usize = 1024;
pub const WRITER_BUFSIZE: usize = 1024;
pub const NUM_WORKERS: usize = 4;
