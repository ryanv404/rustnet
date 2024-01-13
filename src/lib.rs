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

pub mod body;
pub mod client;
pub mod errors;
pub mod headers;
pub mod http;
pub mod io;
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod style;
pub mod utils;
pub mod workers;

pub use body::{Body, Target};
pub use client::{Client, ClientBuilder};
pub use errors::{NetError, NetParseError, NetResult};
pub use headers::{Header, Headers, HeaderName, HeaderValue};
pub use io::{Connection, WriteCliError};
pub use http::{Method, Status, Version};
pub use request::{Request, RequestBuilder, UriPath};
pub use response::{Response, ResponseBuilder};
pub use router::{Route, RouteBuilder, Router};
pub use server::{Listener, Server, ServerBuilder, ServerHandle};
pub use style::{Style, StyleKind, StyleParts};
pub use workers::{ThreadPool, Worker};

pub const READER_BUFSIZE: usize = 2048;
pub const WRITER_BUFSIZE: usize = 2048;
pub const TEST_SERVER_ADDR: &str = "127.0.0.1:7878";
pub const CLIENT_NAME: &str = "http_client";
pub const SERVER_NAME: &str = "http_server";
pub const TUI_NAME: &str = "http_tui";
pub const DEFAULT_NAME: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "/",
    env!("CARGO_PKG_VERSION")
);

/// The maximum number of headers that a `Request` or `Response` can contain
/// in order to mitigate against a denial of service attack.
pub const MAX_HEADERS: u16 = 1024;

#[cfg(test)]
mod tests;
