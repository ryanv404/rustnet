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
pub mod tui;
pub mod util;

pub use body::Body;
pub use client::{Client, ClientBuilder, ClientCli, OutputStyle, Parts, Style};
pub use errors::{NetError, NetParseError, NetResult};
pub use header::{
    Header, HeaderName, HeaderNameInner, HeaderValue, Headers, header_name,
};
pub use io::{Connection, WriteCliError};
pub use http::{Method, Status, StatusCode, Version};
pub use request::{Path, Request, RequestBuilder, RequestLine};
pub use response::{Response, ResponseBuilder, StatusLine};
pub use router::{Route, RouteBuilder, Router, Target};
pub use server::{
    Server, ServerBuilder, ServerCli, ServerConfig, ServerHandle,
};
pub use thread_pool::{ThreadPool, Worker};
pub use tui::Tui;

pub mod colors {
    pub const RED: &str = "\x1b[91m";
    pub const GRN: &str = "\x1b[92m";
    pub const YLW: &str = "\x1b[93m";
    pub const BLU: &str = "\x1b[94m";
    pub const PURP: &str = "\x1b[95m";
    pub const CYAN: &str = "\x1b[96m";
    pub const CLR: &str = "\x1b[0m";
}

pub mod config {
    pub const NUM_WORKERS: usize = 4;
    pub const READER_BUFSIZE: usize = 2048;
    pub const WRITER_BUFSIZE: usize = 2048;
    pub const TEST_SERVER_ADDR: &str = "127.0.0.1:7878";
    pub const DEFAULT_NAME: &str = concat!(
        env!("CARGO_CRATE_NAME"),
        "/",
        env!("CARGO_PKG_VERSION")
    );
}
