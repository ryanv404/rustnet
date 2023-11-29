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

#[cfg(test)]
mod tests;

pub mod client;
pub mod errors;
pub mod header;
pub mod http;
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod util;

pub use client::{Client, ClientBuilder};
pub use errors::{NetError, NetResult, ParseErrorKind};
pub use header::{Header, HeaderKind, HeaderName, Headers, HeaderValue};
pub use http::{Method, Status, Version};
pub use request::{Request, RequestLine, NetReader};
pub use response::{Response, StatusLine, NetWriter};
pub use router::{Body, Route, RouteBuilder, Router, Target};
pub use server::{Server, ServerBuilder, Task, ThreadPool, Worker};
pub use util::{trim_whitespace_bytes, get_datetime};

pub mod consts {
    pub use crate::header::header_consts::*;
    pub const MAX_HEADERS: u16 = 1024;
    pub const NUM_WORKER_THREADS: usize = 4;
    pub const READER_BUFSIZE: usize = 1024;
    pub const WRITER_BUFSIZE: usize = 1024;

    #[cfg(test)]
    pub use crate::header::names::TEST_HEADERS;
}
