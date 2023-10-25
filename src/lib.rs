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

use std::io::{self, BufReader, BufWriter};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::thread::spawn;

pub mod connection;
pub mod error;
pub mod headers;
pub mod http;
pub mod request;
pub mod response;
pub mod router;
pub mod util;
pub mod worker;

pub use error::NetError;
pub use headers::{Header, HeaderName};
pub use http::{Method, Status, Version};
pub use request::Request;
pub use response::Response;
pub use router::Router;

pub type ArcRouter = Arc<Mutex<Router>>;

/// The `Server` object. This is the main entry point to the public API.
pub struct Server {
    local_addr: Option<SocketAddr>,
    router: ArcRouter,
    listener: TcpListener,
}

impl Server {
    /// Returns a `Server` instance bound to the provided IPv4 address and port.
    #[must_use]
    pub fn new<A: ToSocketAddrs>(addr: A) -> Self {
        match TcpListener::bind(addr) {
            Ok(listener) => {
                let router = Arc::new(Mutex::new(Router::new()));
                let local_addr = listener.local_addr().ok();

                Self { local_addr, router, listener }
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

    pub fn local_addr(&self) -> Option<&SocketAddr> {
        self.local_addr.as_ref()
    }

    pub fn local_ip(&self) -> Option<IpAddr> {
        self.local_addr().map(|sock| sock.ip())
    }

    pub fn local_port(&self) -> Option<u16> {
        self.local_addr().map(|sock| sock.port())
    }

    pub fn log_server_start(&self) {
        let maybe_addr = (self.local_ip(), self.local_port());

        if let (Some(ip), Some(port)) = maybe_addr {
            println!("[SERVER] Listening on {ip} at port {port}.");
        } else {
            println!("[SERVER] Unable to determine the local address.");
        }
    }

    pub fn log_server_shutdown(&self) {
        println!("[SERVER] Now shutting down.");
    }

    /// Starts the server.
    pub fn start(&self) -> io::Result<()> {
        self.log_server_start();

        for s in self.listener.incoming() {
            match s {
                Ok(stream) => {
                    let router = self.router.clone();

                    spawn(move || {
                        if let Err(e) = Self::handle_connection(stream, router) {
                            eprintln!("[SERVER] Error: {e}");
                        }
                    }).join().unwrap();
                },
                Err(e) => eprintln!("[SERVER] Incoming stream error: {e}"),
            }
        }

        self.log_server_shutdown();
        Ok(())
    }

    fn handle_connection(stream: TcpStream, router: ArcRouter) -> io::Result<()> {
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut writer = BufWriter::new(stream);

        let req = Request::from_reader(&mut reader)?;
        let res = req.respond(&router)?;

        res.send(&mut writer)?;
        Ok(())
    }
}
