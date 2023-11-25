use std::error::Error as StdError;
use std::io::Result as IoResult;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

use crate::consts::NUM_WORKER_THREADS;
use crate::{
    Method, NetResult, Connection, Request, Response, Route, Router,
    Target, ThreadPool,
};

/// Configures the socket address and the router for a `Server`.
pub struct ServerBuilder<A: ToSocketAddrs> {
    pub addr: A,
    pub router: Router,
    pub do_logging: bool,
}

impl<A: ToSocketAddrs> ServerBuilder<A> {
    /// Creates a `ServerBuilder` object containing the provided socket address.
    pub fn new(addr: A) -> Self {
        Self {
            addr,
            router: Router::new(),
            do_logging: false
        }
    }

    /// Starts the server, returning a handle to the running `Server` instance.
    pub fn start(self) -> IoResult<Server> {
        let Self { addr, router, do_logging } = self;
        let router = Arc::new(router);
        Server::new(addr, router, do_logging)
    }

    /// Configures handling of a GET request.
    pub fn get<P: Into<PathBuf>>(&mut self, uri_path: &str, file_path: P) {
        let route = Route::new(Method::Get, uri_path);
        let target = Target::File(file_path.into());
        self.router.mount(route, target);
    }

    /// Configures handling of a POST request.
    pub fn post(&mut self, uri_path: &str) {
        let route = Route::new(Method::Post, uri_path);
        self.router.mount(route, Target::Empty);
    }

    /// Configures handling of a PUT request.
    pub fn put(&mut self, uri_path: &str) {
        let route = Route::new(Method::Put, uri_path);
        self.router.mount(route, Target::Empty);
    }

    /// Configures handling of a PATCH request.
    pub fn patch(&mut self, uri_path: &str) {
        let route = Route::new(Method::Patch, uri_path);
        self.router.mount(route, Target::Empty);
    }

    /// Configures handling of a DELETE request.
    pub fn delete(&mut self, uri_path: &str) {
        let route = Route::new(Method::Delete, uri_path);
        self.router.mount(route, Target::Empty);
    }

    /// Configures handling of a TRACE request.
    pub fn trace(&mut self, uri_path: &str) {
        let route = Route::new(Method::Trace, uri_path);
        self.router.mount(route, Target::Empty);
    }

    /// Configures handling of a CONNECT request.
    pub fn connect(&mut self, uri_path: &str) {
        let route = Route::new(Method::Connect, uri_path);
        self.router.mount(route, Target::Empty);
    }

    /// Configures handling of an OPTIONS request.
    pub fn options(&mut self, uri_path: &str) {
        let route = Route::new(Method::Options, uri_path);
        self.router.mount(route, Target::Empty);
    }

    /// Sets the static file path to a favicon icon.
    pub fn set_favicon<P: Into<PathBuf>>(&mut self, file_path: P) {
        let route = Route::new(Method::Get, "/favicon.ico");
        let target = Target::File(file_path.into());
        self.router.mount(route, target);
    }

    /// Sets the static file path to an HTML page returned by 404 responses.
    pub fn set_error_page<P: Into<PathBuf>>(&mut self, file_path: P) {
        let route = Route::new(Method::Get, "__error");
        let target = Target::File(file_path.into());
        self.router.mount(route, target);
    }

    /// Enables logging of request lines and status codes to stdout.
    pub fn enable_logging(&mut self) {
        self.do_logging = true;
    }
}

/// A wrapper around a `TcpListener` instance.
pub struct Listener {
    pub inner: TcpListener,
}

impl From<TcpListener> for Listener {
    fn from(inner: TcpListener) -> Self {
        Self { inner }
    }
}

impl Listener {
    /// Bind the listener to the given socket address.
    pub fn bind<A: ToSocketAddrs>(addr: A) -> IoResult<Self> {
        let inner = TcpListener::bind(addr)?;
        Ok(Self { inner })
    }

    /// Returns the server's socket address.
    pub fn local_addr(&self) -> IoResult<SocketAddr> {
        self.inner.local_addr()
    }

    /// Returns a `Connection` instance for each incoming connection.
    pub fn accept(&self) -> IoResult<Connection> {
        self.inner.accept().and_then(Connection::try_from)
    }
}

/// The `Server` object. This is the main entry point to the public API.
pub struct Server {
    /// Handle for the server's listening thread.
    pub handle: JoinHandle<()>,
    /// The server's socket address.
    pub local_addr: SocketAddr,
    /// Trigger for closing the server.
    pub keep_listening: Arc<AtomicBool>,
    /// Flag to enable logging of each connection to stdout.
    pub do_logging: Arc<AtomicBool>,
}

impl Server {
    /// Returns a `ServerBuilder` containing the provided socket address.
    pub fn http<A: ToSocketAddrs>(addr: A) -> ServerBuilder<A> {
        ServerBuilder::new(addr)
    }

    /// Returns a `Server` instance this is bound to the given address.
    pub fn new<A: ToSocketAddrs>(
        addr: A,
        router: Arc<Router>,
        enable_logging: bool
    ) -> IoResult<Self> {
        let listener = Listener::bind(addr)?;
        let local_addr = listener.local_addr()?;

        let do_logging = Arc::new(AtomicBool::new(enable_logging));
        let do_log = Arc::clone(&do_logging);

        let keep_listening = Arc::new(AtomicBool::new(true));
        let listening = Arc::clone(&keep_listening);

        if do_logging.load(Ordering::Relaxed) {
            Self::log_start_up(&local_addr);
        }

        // Spawn listener thread.
        let handle = spawn(move || {
            // Create a thread pool to handle incoming requests.
            let pool = ThreadPool::new(NUM_WORKER_THREADS);

            while listening.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(conn) => {
                        let rtr = Arc::clone(&router);
                        let log = Arc::clone(&do_log);

                        // Task an available worker thread with responding.
                        pool.execute(move || {
                            if let Err(e) = Self::respond(conn, &rtr, &log) {
                                Self::log_error(&e);
                            }
                        });
                    },
                    Err(e) => Self::log_error(&e),
                }
            }

            if do_log.load(Ordering::Relaxed) {
                Self::log_shutdown();
            }
        });

        Ok(Self {
            handle,
            local_addr,
            keep_listening,
            do_logging
        })
    }

    /// Handles a request from a remote connection.
    pub fn respond(
        conn: Connection,
        router: &Arc<Router>,
        do_logging: &Arc<AtomicBool>
    ) -> NetResult<()> {
        let cloned_conn = conn.try_clone()?;
        let req = Request::try_from(cloned_conn)?;
        let resolved = router.resolve(&req);

        if do_logging.load(Ordering::Relaxed) {
            Self::log_with_status(
                conn.remote_ip(),
                resolved.status.code(),
                &req
            );
        }

        let mut res = Response::from_request(&req, &resolved, conn)?;
        res.send()?;
        Ok(())
    }

    /// Returns the local socket address of the server.
    #[must_use]
    pub const fn local_addr(&self) -> &SocketAddr {
        &self.local_addr
    }

    /// Returns the local IP address of the server.
    #[must_use]
    pub const fn local_ip(&self) -> IpAddr {
        self.local_addr.ip()
    }

    /// Returns the local port of the server.
    #[must_use]
    pub const fn local_port(&self) -> u16 {
        self.local_addr.port()
    }

    /// Logs the response status and request line.
    pub fn log_with_status(ip: IpAddr, status: u16, req: &Request) {
        println!("[{ip}|{status}] {} {}", req.method(), req.path());
    }

    /// Logs a non-terminating server error.
    pub fn log_error(e: &dyn StdError) {
        eprintln!("[SERVER] ERROR: {e}");
    }

    /// Logs a server start up message to stdout.
    pub fn log_start_up(addr: &SocketAddr) {
        let ip = addr.ip();
        let port = addr.port();
        eprintln!("[SERVER] Listening on {ip} at port {port}.");
    }

    /// Logs a server shutdown message to stdout.
    pub fn log_shutdown() {
        eprintln!("[SERVER] Now shutting down.");
    }

    /// Triggers graceful shutdown of the server.
    pub fn shutdown(&self) {
        Self::log_shutdown();

        // Stops the listener thread's loop.
        self.keep_listening.store(false, Ordering::Relaxed);

        // Briefly connect to ourselves to unblock the listener thread.
        if let Ok(stream) = TcpStream::connect(self.local_addr) {
            stream.shutdown(Shutdown::Both).unwrap();
        }

        // Give the worker threads a bit of time to shutdown.
        thread::sleep(Duration::from_millis(200));
    }
}
