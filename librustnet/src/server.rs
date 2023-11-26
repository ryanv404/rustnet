use std::error::Error as StdError;
use std::io::ErrorKind as IoErrorKind;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

use crate::consts::NUM_WORKER_THREADS;
use crate::{
    Method, NetError, NetResult, Connection, Request, Response, Route, Router,
    Target, ThreadPool,
};

/// Configures the socket address and the router for a `Server`.
#[derive(Debug)]
pub struct ServerBuilder<A>
where
    A: ToSocketAddrs
{
    pub addr: Option<A>,
    pub ip: Option<IpAddr>,
    pub port: Option<u16>,
    pub router: Router,
    pub do_logging: bool,
}

impl<A> Default for ServerBuilder<A>
where
    A: ToSocketAddrs
{
    fn default() -> Self {
        Self {
            addr: None,
            ip: None,
            port: None,
            router: Router::new(),
            do_logging: false,
        }
    }
}

impl<A> ServerBuilder<A>
where
    A: ToSocketAddrs
{
    /// Returns a builder object that is used to build a `Server`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds and returns a `Server` instance.
    pub fn build(self) -> NetResult<Server> {
        if let Some(addr) = self.addr.as_ref() {
            let listener = Listener::bind(addr)?;

            let server = Server {
                router: Arc::new(self.router),
                listener: Arc::new(listener),
                do_logging: Arc::new(AtomicBool::new(self.do_logging)),
                keep_listening: Arc::new(AtomicBool::new(false))
            };

            return Ok(server);
        }

        if let (Some(ip), Some(port)) = (self.ip, self.port) {
            let addr = format!("{ip}:{port}");
            let listener = Listener::bind(&addr)?;

            let server = Server {
                router: Arc::new(self.router),
                listener: Arc::new(listener),
                do_logging: Arc::new(AtomicBool::new(self.do_logging)),
                keep_listening: Arc::new(AtomicBool::new(false))
            };

            return Ok(server);
        }

        Err(NetError::IoError(IoErrorKind::InvalidInput))
    }

    /// Sets the server's IP address.
    pub fn ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }

    /// Sets the server's port.
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Sets the server's socket address.
    pub fn addr(mut self, addr: A) -> Self {
        self.addr = Some(addr);
        self
    }

    /// Configures handling of a server end-point.
    pub fn route(self, uri_path: &str) -> Self {
        let _route = Route::new(Method::Get, uri_path);
        self
    }

    /// Configures handling of a GET request.
    pub fn get<P>(mut self, uri_path: &str, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, uri_path);
        let target = Target::File(file_path.into());
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a GET request.
    pub fn get_with_handler<F>(mut self, uri_path: &str, handler: F) -> Self
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Get, uri_path);
        let target = Target::Handler(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    pub fn post(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Post, uri_path);
        self.router.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a PUT request.
    pub fn put(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Put, uri_path);
        self.router.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a PATCH request.
    pub fn patch(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Patch, uri_path);
        self.router.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a DELETE request.
    pub fn delete(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Delete, uri_path);
        self.router.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a TRACE request.
    pub fn trace(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Trace, uri_path);
        self.router.mount(route, Target::Empty);
        self

    }

    /// Configures handling of a CONNECT request.
    pub fn connect(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Connect, uri_path);
        self.router.mount(route, Target::Empty);
        self

    }

    /// Configures handling of an OPTIONS request.
    pub fn options(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Options, uri_path);
        self.router.mount(route, Target::Empty);
        self

    }

    /// Sets the static file path to a favicon icon.
    pub fn set_favicon<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, "/favicon.ico");
        let target = Target::File(file_path.into());
        self.router.mount(route, target);
        self
    }

    /// Sets the static file path to an HTML page returned by 404 responses.
    pub fn set_error_page<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, "__error");
        let target = Target::File(file_path.into());
        self.router.mount(route, target);
        self
    }

    /// Enables logging of request lines and status codes to stdout.
    pub fn enable_logging(&mut self) {
        self.do_logging = true;
    }
}

/// A wrapper around a `TcpListener` instance.
#[derive(Debug)]
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
    pub fn bind<A>(addr: A) -> NetResult<Self>
    where
        A: ToSocketAddrs
    {
        let inner = TcpListener::bind(addr)?;
        Ok(Self { inner })
    }

    /// Returns the server's socket address.
    pub fn local_addr(&self) -> NetResult<SocketAddr> {
        self.inner.local_addr().map_err(|e| e.into())
    }

    /// Returns a `Connection` instance for each incoming connection.
    pub fn accept(&self) -> NetResult<Connection> {
        self.inner.accept()
            .and_then(Connection::try_from)
            .map_err(|e| e.into())
    }
}

/// A handle that is returned when a server starts.
#[derive(Debug)]
pub struct ServerHandle {
    /// Handle for the server's listening thread.
    pub thread: JoinHandle<()>,
    /// Trigger for closing the server.
    pub keep_listening: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct Server {
    /// Router containing the handler logic for server end-points.
    pub router: Arc<Router>,
    /// The local socket on which the server listens.
    pub listener: Arc<Listener>,
    /// Flag to enable logging of each connection to stdout.
    pub do_logging: Arc<AtomicBool>,
    /// Trigger for closing the server.
    pub keep_listening: Arc<AtomicBool>,
}

impl Server {
    /// Returns a builder object that is used to build a `Server`.
    pub fn builder<A>() -> ServerBuilder<A>
    where
        A: ToSocketAddrs
    {
        ServerBuilder::default()
    }

    /// Returns a `Server` instance bound to the provided socket address.
    pub fn http<A>(addr: A) -> NetResult<Self>
    where
        A: ToSocketAddrs
    {
        let listener = Listener::bind(addr)?;
        
        Ok(Self {
            router: Arc::new(Router::new()),
            listener: Arc::new(listener),
            do_logging: Arc::new(AtomicBool::new(false)),
            keep_listening: Arc::new(AtomicBool::new(false))
        })
    }

    /// Activates the server to start listening on its bound address.
    pub fn start(self) -> NetResult<ServerHandle> {
        let router = Arc::clone(&self.router);
        let listener = Arc::clone(&self.listener);
        let do_logging = Arc::clone(&self.do_logging);
        let keep_listening = Arc::clone(&self.keep_listening);

        if do_logging.load(Ordering::Relaxed) {
            Self::log_start_up(&listener.local_addr()?);
        }

        self.keep_listening.store(true, Ordering::Relaxed);

        // Spawn listener thread.
        let handle = spawn(move || {
            // Create a thread pool to handle incoming requests.
            let pool = ThreadPool::new(NUM_WORKER_THREADS);

            while keep_listening.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(conn) => {
                        let rtr = Arc::clone(&router);
                        let do_log = Arc::clone(&do_logging);

                        // Task an available worker thread with responding.
                        pool.execute(move || {
                            if let Err(e) = Self::respond(conn, &rtr, &do_log) {
                                Self::log_error(&e);
                            }
                        });
                    },
                    Err(e) => Self::log_error(&e),
                }
            }

            if do_logging.load(Ordering::Relaxed) {
                Self::log_shutdown();
            }
        });

        Ok(ServerHandle {
            thread: handle,
            keep_listening: self.keep_listening
        })
    }

    /// Handles a request from a remote connection.
    pub fn respond(
        conn: Connection,
        router: &Arc<Router>,
        do_logging: &Arc<AtomicBool>
    ) -> NetResult<()> {
        let req = Request::try_from(conn)?;
        let method = req.request_line.method;
        let path = req.request_line.path.clone();

        let mut res = Router::resolve(req, router)?;

        if do_logging.load(Ordering::Relaxed) {
            let ip = res.remote_ip();
            let status = res.status_code();

            Self::log_with_status(ip, status, method, &path);
        }

        res.send()?;

        Ok(())
    }

    /// Returns the local socket address of the server.
    #[must_use]
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    /// Returns the local IP address of the server.
    #[must_use]
    pub fn local_ip(&self) -> Option<IpAddr> {
        self.local_addr().map_or(None, |sock| Some(sock.ip()))
    }

    /// Returns the local port of the server.
    #[must_use]
    pub fn local_port(&self) -> Option<u16> {
        self.local_addr().map_or(None, |sock| Some(sock.port()))
    }

    /// Logs the response status and request line.
    pub fn log_with_status(ip: IpAddr, status: u16, method: Method, path: &str) {
        println!("[{ip}|{status}] {method} {path}");
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
        self.local_addr()
            .map(|addr| {
                if let Ok(stream) = TcpStream::connect(addr) {
                    stream.shutdown(Shutdown::Both).unwrap();
                }
            });

        // Give the worker threads a bit of time to shutdown.
        thread::sleep(Duration::from_millis(200));
    }
}
