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
pub mod style;
pub mod thread_pool;
pub mod tui;
pub mod util;

pub use body::{Body, Target};
pub use client::{Client, ClientBuilder, ClientCli};
pub use errors::{NetError, NetParseError, NetResult};
pub use header::{
    Header, Headers, HeaderName, HeaderNameInner, HeaderValue,
};
pub use io::{Connection, WriteCliError};
pub use http::{Method, Status, Version};
pub use request::{Request, RequestBuilder, RequestLine, UriPath};
pub use response::{Response, ResponseBuilder, StatusLine};
pub use router::{Route, RouteBuilder, Router};
pub use server::{
    Server, ServerBuilder, ServerCli, ServerHandle,
};
pub use style::{Style, StyleKind, StyleParts};
pub use thread_pool::{ThreadPool, Worker};
pub use tui::Tui;

pub const TEST_SERVER_ADDR: &str = "127.0.0.1:7878";
pub const DEFAULT_NAME: &str = concat!(
    env!("CARGO_CRATE_NAME"),
    "/",
    env!("CARGO_PKG_VERSION")
);
