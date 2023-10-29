use std::io::Result as IoResult;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

use crate::{
    Method, NetError, NetResult, RemoteClient, Request, Response, Route,
	Router, ThreadPool,
};
use crate::consts::NUM_WORKERS;

/// Configures the socket address and the router for a `Server`.
#[allow(clippy::module_name_repetitions)]
pub struct ServerConfig<A: ToSocketAddrs> {
    /// User-provided socket address.
    pub addr: A,
    /// The server's router.
    pub router: Router,
}

impl<A: ToSocketAddrs> ServerConfig<A> {
    /// Creates a `ServerConfig` object containing the provided socket address.
    pub fn new(addr: A) -> Self {
        Self {
            addr,
            router: Router::new(),
        }
    }

    /// Starts the server, returning a handle to the running `Server` instance.
    pub fn start(self) -> IoResult<Server> {
        let Self { addr, router } = self;
        let router = Arc::new(router);
        Server::new(addr, router)
    }

    /// Configures handling of a GET request for the provided URI. Mounts the local
    /// resource at the provided path to this URI.
    pub fn get<P: Into<PathBuf>>(&mut self, uri: &str, path: P) {
        let route = Route::new(Method::Get, uri);
        self.router.mount_with_path(route, path);
    }

    /// Configures handling of POST requests for the provided URI.
    pub fn post(&mut self, uri: &str) {
        let route = Route::new(Method::Post, uri);
        self.router.mount(route);
    }

    /// Configures handling of PUT requests for the provided URI.
    pub fn put(&mut self, uri: &str) {
        let route = Route::new(Method::Put, uri);
        self.router.mount(route);
    }

    /// Configures handling of PATCH requests for the provided URI.
    pub fn patch(&mut self, uri: &str) {
        let route = Route::new(Method::Patch, uri);
        self.router.mount(route);
    }

    /// Configures handling of DELETE requests for the provided URI.
    pub fn delete(&mut self, uri: &str) {
        let route = Route::new(Method::Delete, uri);
        self.router.mount(route);
    }

    /// Configures handling of HEAD requests for the provided URI.
    pub fn head(&mut self, uri: &str) {
        let route = Route::new(Method::Head, uri);
        self.router.mount(route);
    }

    /// Configures handling of CONNECT requests.
    #[allow(dead_code)]
    fn connect(&mut self) {
        todo!();
    }

    /// Configures handling of OPTIONS requests.
    #[allow(dead_code)]
    fn options(&mut self) {
        todo!();
    }

    /// Configures handling of TRACE requests.
    #[allow(dead_code)]
    fn trace(&mut self) {
        todo!();
    }

    /// Sets the local path to the favicon.
    pub fn set_favicon<P: Into<PathBuf>>(&mut self, path: P) {
        self.get("/favicon.ico", path);
    }

    /// Sets the local path to an HTML page returned in error responses.
    pub fn set_error_page<P: Into<PathBuf>>(&mut self, path: P) {
        self.get("__error", path);
    }
}

pub struct Listener {
    pub inner: TcpListener,
}

impl From<TcpListener> for Listener {
    fn from(inner: TcpListener) -> Self {
        Self { inner }
    }
}

impl Listener {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> IoResult<Self> {
        let inner = TcpListener::bind(addr)?;
        Ok(Self { inner })
    }

    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr()
    }

    pub fn accept(&self) -> IoResult<RemoteClient> {
        self.inner.accept().and_then(RemoteClient::try_from)
    }
}

/// The `Server` object. This is the main entry point to the public API.
pub struct Server {
    /// Thread handle for the server's listening thread.
    pub handle: JoinHandle<()>,
    /// The server's socket address.
    pub local_addr: SocketAddr,
    /// Trigger for closing the server.
    pub close_trigger: Arc<AtomicBool>,
}

impl Server {
    /// Returns a `ServerConfig` containing the provided socket address.
    pub fn http<A: ToSocketAddrs>(addr: A) -> ServerConfig<A> {
        ServerConfig::new(addr)
    }

    /// Returns a running `Server` instance.
    pub fn new<A: ToSocketAddrs>(addr: A, router: Arc<Router>) -> IoResult<Self> {
        // Get server listener.
        let listener = Listener::bind(addr)?;

        // Local server address.
        let local_addr = listener.local_addr()?;

        // Initialize server close trigger.
        let close_trigger = Arc::new(AtomicBool::new(true));
        let listener_running = Arc::clone(&close_trigger);

        Self::log_server_start(&local_addr);

        // Spawns a thread that listens for new incoming connections.
        let handle = spawn(move || {
            let pool = ThreadPool::new(NUM_WORKERS);

            while listener_running.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(client) => {
                        let rtr = Arc::clone(&router);

                        pool.execute(move || {
                            if let Err(e) = Self::respond(client, &rtr) {
                                let _ = e;
                            }
                        });
                    }
                    Err(e) => {
                        let _ = NetError::from_kind(e.kind());
                    }
                }
            }
        });

        Ok(Self {
            handle,
            local_addr,
            close_trigger,
        })
    }

    /// Handles a remote client connection.
    pub fn respond(mut client: RemoteClient, router: &Arc<Router>) -> NetResult<()> {
        let req = Request::from_client(&mut client)?;
        let res = Response::from_request(&req, router)?;
        res.send(&mut client)?;
        Ok(())
    }

    /// Returns the IP address on which the server is listening.
    #[must_use]
    pub const fn local_ip(&self) -> IpAddr {
        self.local_addr.ip()
    }

    /// Returns the port on which the server is listening.
    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.local_addr.port()
    }

    /// Logs a start up message to stdout.
    pub fn log_server_start(addr: &SocketAddr) {
        println!(
            "[SERVER] Listening on {} at port {}.",
            addr.ip(),
            addr.port()
        );
    }

    /// Logs a shutdown message to stdout.
    pub fn log_server_shutdown(&self) {
        println!("[SERVER] Now shutting down.");
    }

    /// Triggers graceful shutdown of the server.
    pub fn shutdown(&self) {
        self.log_server_shutdown();

		// Stops the listener thread's loop.
        self.close_trigger.store(false, Ordering::Relaxed);

        // Connect to and unblock the listener thread.
        if let Ok(stream) = TcpStream::connect(self.local_addr) {
            stream.shutdown(Shutdown::Both).unwrap();
        }

        // Give worker threads time to shutdown.
        thread::sleep(Duration::from_millis(100));
    }
}
