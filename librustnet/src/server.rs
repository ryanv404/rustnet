use std::convert::Into;
use std::error::Error as StdError;
use std::io::ErrorKind as IoErrorKind;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

use crate::consts::NUM_WORKER_THREADS;
use crate::{
    Method, NetError, NetResult, Connection, Request, Response, Route,
    Router, Target,
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
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds and returns a `Server` instance.
    pub fn start(self) -> NetResult<ServerHandle> {
        if let Some(addr) = self.addr.as_ref() {
            let listener = Listener::bind(addr)?;

            let server = Server {
                router: Arc::new(self.router),
                listener: Arc::new(listener),
                do_logging: Arc::new(AtomicBool::new(self.do_logging)),
                keep_listening: Arc::new(AtomicBool::new(false))
            };

            return server.start();
        }

        if let (Some(ip), Some(port)) = (self.ip, self.port) {
            let addr = format!("{ip}:{port}");
            let listener = Listener::bind(addr)?;

            let server = Server {
                router: Arc::new(self.router),
                listener: Arc::new(listener),
                do_logging: Arc::new(AtomicBool::new(self.do_logging)),
                keep_listening: Arc::new(AtomicBool::new(false))
            };

            return server.start();
        }

        Err(NetError::IoError(IoErrorKind::InvalidInput))
    }

    /// Sets the server's IP address.
    #[must_use]
    pub const fn ip(mut self, ip: IpAddr) -> Self {
        self.ip = Some(ip);
        self
    }

    /// Sets the server's port.
    #[must_use]
    pub const fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Sets the server's socket address.
    #[must_use]
    pub fn addr(mut self, addr: A) -> Self {
        self.addr = Some(addr);
        self
    }

    /// Configures handling of a server end-point.
    #[must_use]
    pub fn route(self, uri_path: &str) -> Self {
        let _route = Route::new(Method::Get, uri_path);
        self
    }

    /// Configures handling of a GET request.
    #[must_use]
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
    #[must_use]
    pub fn get_with_handler<F>(mut self, uri_path: &str, handler: F) -> Self
    where
        F: FnMut(&Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(Method::Get, uri_path);
        let target = Target::FnMut(Arc::new(Mutex::new(handler)));
        self.router.mount(route, target);
        self
    }

    /// Configures handling of a POST request.
    #[must_use]
    pub fn post(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Post, uri_path);
        self.router.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a PUT request.
    #[must_use]
    pub fn put(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Put, uri_path);
        self.router.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a PATCH request.
    #[must_use]
    pub fn patch(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Patch, uri_path);
        self.router.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a DELETE request.
    #[must_use]
    pub fn delete(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Delete, uri_path);
        self.router.mount(route, Target::Empty);
        self
    }

    /// Configures handling of a TRACE request.
    #[must_use]
    pub fn trace(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Trace, uri_path);
        self.router.mount(route, Target::Empty);
        self

    }

    /// Configures handling of a CONNECT request.
    #[must_use]
    pub fn connect(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Connect, uri_path);
        self.router.mount(route, Target::Empty);
        self

    }

    /// Configures handling of an OPTIONS request.
    #[must_use]
    pub fn options(mut self, uri_path: &str) -> Self {
        let route = Route::new(Method::Options, uri_path);
        self.router.mount(route, Target::Empty);
        self

    }

    /// Sets the static file path to a favicon icon.
    #[must_use]
    pub fn favicon<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, "/favicon.ico");
        let target = Target::File(file_path.into());
        self.router.mount(route, target);
        self
    }

    /// Sets the static file path to an HTML page returned by 404 responses.
    #[must_use]
    pub fn error_page<P>(mut self, file_path: P) -> Self
    where
        P: Into<PathBuf>
    {
        let route = Route::new(Method::Get, "__error");
        let target = Target::File(file_path.into());
        self.router.mount(route, target);
        self
    }

    /// Enables logging of request lines and status codes to stdout.
    pub fn logging(&mut self, do_log: bool) {
        self.do_logging = do_log;
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
        self.inner.local_addr().map_err(Into::into)
    }

    /// Returns a `Connection` instance for each incoming connection.
    pub fn accept(&self) -> NetResult<Connection> {
        self.inner.accept()
            .map_err(|e| NetError::ReadError(e.kind()))
            .and_then(|(stream, addr)|Connection::try_from((stream, addr)))
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
    #[must_use]
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
            Self::log_start_up(listener.local_addr()?);
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
        let reader = conn.reader.try_clone()?;
        let mut req = Request::recv(reader)?;
        let mut res = router.resolve(&mut req)?;

        if do_logging.load(Ordering::Relaxed) {
            Self::log_with_status(
                res.remote_ip(),
                res.status_code(),
                req.method(),
                req.path()
            );
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
        self.local_addr().map(|sock| sock.ip())
    }

    /// Returns the local port of the server.
    #[must_use]
    pub fn local_port(&self) -> Option<u16> {
        self.local_addr().map(|sock| sock.port())
    }

    /// Logs the response status and request line.
    pub fn log_with_status(
        maybe_ip: Option<IpAddr>,
        status: u16,
        method: Method,
        path: &str
    ) {
        match maybe_ip {
            Some(ip) => println!("[{ip}|{status}] {method} {path}"),
            None => println!("[?|{status}] {method} {path}"),
        }
    }

    /// Logs a non-terminating server error.
    pub fn log_error(e: &dyn StdError) {
        eprintln!("[SERVER] ERROR: {e}");
    }

    /// Logs a server start up message to stdout.
    pub fn log_start_up(addr: SocketAddr) {
        let ip = addr.ip();
        let port = addr.port();
        eprintln!("[SERVER] Listening on {ip} at port {port}.");
    }

    /// Logs a server shutdown message to stdout.
    pub fn log_shutdown() {
        eprintln!("[SERVER] Now shutting down.");
    }

    /// Triggers graceful shutdown of the server.
    pub fn shutdown(&self) -> NetResult<()> {
        // Stops the listener thread's loop.
        self.keep_listening.store(false, Ordering::Relaxed);

        // Briefly connect to ourselves to unblock the listener thread.
        if let Some(addr) = self.local_addr() {
            let _ = TcpStream::connect(addr).map(|stream|
                stream.shutdown(Shutdown::Both));
        }

        // Give the worker threads a bit of time to shutdown.
        thread::sleep(Duration::from_millis(200));
        Ok(())
    }
}

pub type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct Worker {
    _id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Spawns a worker thread that receives tasks and executes them.
    fn new(_id: usize, receiver: Arc<Mutex<Receiver<Task>>>) -> Self {
        let handle = thread::spawn(move || {
            while let Ok(job) = receiver.lock().unwrap().recv() {
                job();
            }
        });

        Self { _id, handle: Some(handle) }
    }
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<Sender<Task>>,
}

impl ThreadPool {
    /// Create a new `ThreadPool` with the given number of worker threads.
    #[must_use]
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let mut workers = Vec::with_capacity(size);
        let (tx, rx) = channel();

        let sender = Some(tx);
        let receiver = Arc::new(Mutex::new(rx));

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self { workers, sender }
    }

    /// Sends a `Task` to a worker thread for executon.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .as_ref()
            .unwrap()
            .send(Box::new(f))
            .unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(handle) = worker.handle.take() {
                handle.join().unwrap();
            }
        }
    }
}
