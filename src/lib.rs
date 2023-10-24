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

use std::error;
use std::fmt;
use std::io::{self, BufReader, BufWriter, ErrorKind};
use std::net::{IpAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread;

pub mod connection;
pub mod headers;
pub mod http;
pub mod pool;
pub mod request;
pub mod response;
pub mod router;
pub mod util;

use headers::{Header, HeaderName};
use http::{Method, Status, Version};
use request::Request;
use response::Response;
use router::Router;

/// The `Server` object. This is the main entry point to the public API.
pub struct Server {
    local_ip: IpAddr,
    local_port: u16,
    router: Arc<Mutex<Router>>,
    listener: TcpListener,
}

impl Server {
    /// Returns a `Server` instance bound to the provided IPv4 address and port.
    #[must_use]
    pub fn new<A: ToSocketAddrs>(addr: A) -> Self {
        match TcpListener::bind(addr) {
            Ok(listener) => {
                let router = Arc::new(Mutex::new(Router::new()));

                let sock = listener.local_addr().unwrap();
                let local_ip = sock.ip();
                let local_port = sock.port();

                Self { local_ip, local_port, router, listener }
            },
            Err(e) => panic!("Unable to bind to the address.\n{e}"),
        }
    }

    pub fn get(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Get, target, path);
    }

    pub fn post(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Post, target, path);
    }

    pub fn put(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Put, target, path);
    }

    pub fn patch(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Patch, target, path);
    }

    pub fn delete(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Delete, target, path);
    }

    pub fn head(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Head, target, path);
    }

    pub fn connect(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Connect, target, path);
    }

    pub fn options(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Options, target, path);
    }

    pub fn trace(&mut self, target: &str, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.mount(Method::Trace, target, path);
    }

    pub fn set_favicon(&mut self, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.set_favicon(path);
    }

    pub fn set_error_page(&mut self, path: &str) {
        let mut lock = self.router.lock().unwrap();
        lock.set_error_page(path);
    }

    pub fn ip(&self) -> &IpAddr {
        &self.local_ip
    }

    pub fn port(&self) -> u16 {
        self.local_port
    }

    /// Starts the server.
    pub fn start(&self) -> io::Result<()> {
        println!("[SERVER] Listening on {} at port {}.", self.ip(), self.port());

        for s in self.listener.incoming() {
            match s {
                Ok(stream) => {
                    let router = self.router.clone();

                    thread::spawn(move || {
                        match Server::handle_connection(stream, router) {
                            Err(e) => eprintln!("[SERVER] Error: {e}"),
                            Ok(_) => {},
                        }
                    }).join().unwrap();
                },
                Err(e) => eprintln!("[SERVER] Incoming stream error: {e}"),
            }
        }

        println!("[SERVER] Now shutting down.");
        Ok(())
    }

    fn handle_connection(stream: TcpStream, router: Arc<Mutex<Router>>) -> io::Result<()> {
        let remote_ip = stream.peer_addr()?.ip();
        let s_clone = stream.try_clone()?;

        let mut reader = BufReader::new(stream);
        let mut writer = BufWriter::new(s_clone);

        let req = Request::from_reader(&mut reader)?;
        let res = Response::from_request(&req, &router)?;

        let code = res.status_code();
        let method = req.method();
        let uri = req.uri();
        eprintln!("[{remote_ip}|{code}] {method} {uri}");

        res.send(&mut writer)?;
        Ok(())
    }
}

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
