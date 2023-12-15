use std::convert::Into;
use std::error::Error as StdError;
use std::io::ErrorKind as IoErrorKind;
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

use crate::consts::NUM_WORKER_THREADS;
use crate::{
    Body, Method, NetError, NetReader, NetResult, NetWriter, Request, Response,
    Route, Router, Target,
};

/// Configures the socket address and the router for a `Server`.
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct ServerBuilder<A>
where
    A: ToSocketAddrs
{
    pub ip: Option<IpAddr>,
    pub port: Option<u16>,
    pub addr: Option<A>,
    pub router: Option<Router>,
    pub do_logging: bool,
    pub use_shutdown_route: bool,
}

impl<A> Default for ServerBuilder<A>
where
    A: ToSocketAddrs
{
    fn default() -> Self {
        Self {
            ip: None,
            port: None,
            addr: None,
            router: None,
            do_logging: false,
            use_shutdown_route: false
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

    /// Sets the router for the server.
    #[must_use]
    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Set whether to log connections to stdout (default: disabled).
    #[must_use]
    pub const fn log(mut self, do_logging: bool) -> Self {
        self.do_logging = do_logging;
        self
    }

    /// Set whether to add a route to gracefully shutdown the server
    /// (default: disabled).
    #[must_use]
    pub const fn shutdown_route(mut self, use_shutdown_route: bool) -> Self {
        self.use_shutdown_route = use_shutdown_route;
        self
    }

    /// Builds and returns a `Server` instance.
    #[allow(clippy::missing_errors_doc)]
    pub fn build(mut self) -> NetResult<Server> {
        let mut router = self.router.take().unwrap_or_default();

        if self.use_shutdown_route {
            let route = Route::new(Method::Delete, "/__shutdown_server__");
            let target = Target::Text("The server is now shutting down.");
            router.mount(route, target);
        }

        let listener = self.addr
            .as_ref()
            .and_then(|addr| match Listener::bind(addr) {
                Ok(listener) => Some(listener),
                Err(_) => match (self.ip, self.port) {
                    (Some(ip), Some(port)) => {
                        Listener::bind_ip_port(ip, port).ok()
                    },
                    (_, _) => None,
                },
            })
            .ok_or(IoErrorKind::InvalidInput)?;

        Ok(Server {
            router: Arc::new(router),
            listener: Arc::new(listener),
            do_logging: Arc::new(self.do_logging),
            use_shutdown_route: Arc::new(self.use_shutdown_route),
            keep_listening: Arc::new(AtomicBool::new(false)),
            handle: None
        })
    }

    /// Builds and starts the server.
    #[allow(clippy::missing_errors_doc)]
    pub fn start(self) -> NetResult<Server> {
        let server = self.build()?;
        server.start()
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
    #[allow(clippy::missing_errors_doc)]
    pub fn bind<A>(addr: A) -> NetResult<Self>
    where
        A: ToSocketAddrs
    {
        let inner = TcpListener::bind(addr)?;
        Ok(Self { inner })
    }

    /// Bind the listener to the given socket address.
    #[allow(clippy::missing_errors_doc)]
    pub fn bind_ip_port(ip: IpAddr, port: u16) -> NetResult<Self> {
        let inner = TcpListener::bind((ip, port))?;
        Ok(Self { inner })
    }

    /// Returns the server's socket address.
    #[allow(clippy::missing_errors_doc)]
    pub fn local_addr(&self) -> NetResult<SocketAddr> {
        self.inner
            .local_addr()
            .map_err(Into::into)
    }

    /// Returns a `NetReader` instance for each incoming connection.
    #[allow(clippy::missing_errors_doc)]
    pub fn accept(&self) -> NetResult<(NetReader, NetWriter)> {
        self.inner
            .accept()
            .and_then(|(stream, _)| stream
                .try_clone()
                .map(|cloned| (stream, cloned)))
            .map(|(stream, cloned)| {
                let reader = NetReader::from(stream);
                let writer = NetWriter::from(cloned);
                (reader, writer)
            })
            .map_err(|e| NetError::ReadError(e.kind()))
    }
}

#[derive(Debug)]
pub struct Server {
    /// Router containing the handler logic for server end-points.
    pub router: Arc<Router>,
    /// The local socket on which the server listens.
    pub listener: Arc<Listener>,
    /// Enables logging new connections.
    pub do_logging: Arc<bool>,
    /// Enables use of a route for closing the server.
    pub use_shutdown_route: Arc<bool>,
    /// Trigger for stopping the server's listener loop.
    pub keep_listening: Arc<AtomicBool>,
    /// A handle to the server's listener thread.
    pub handle: Option<JoinHandle<()>>,
}

impl Server {
    /// Returns a `ServerBuilder` object.
    #[must_use]
    pub fn builder<A>() -> ServerBuilder<A>
    where
        A: ToSocketAddrs
    {
        ServerBuilder::new()
    }

    /// Returns a `ServerBuilder` object with the address field set.
    #[must_use]
    pub fn http<A>(addr: A) -> ServerBuilder<A>
    where
        A: ToSocketAddrs
    {
        ServerBuilder::new().addr(addr)
    }

    /// Returns a new `Server` instance.
    #[must_use]
    pub fn new(
        router: Router,
        listener: Listener,
        use_shutdown_route: bool,
        do_log: bool
    ) -> Self {
        Self {
            router: Arc::new(router),
            listener: Arc::new(listener),
            do_logging: Arc::new(do_log),
            use_shutdown_route: Arc::new(use_shutdown_route),
            keep_listening: Arc::new(AtomicBool::new(false)),
            handle: None
        }
    }

    /// Activates the server to start listening on its bound address.
    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub fn start(mut self) -> NetResult<Self> {
        if *self.do_logging {
            let local_addr = self.listener.local_addr()?;
            let ip = local_addr.ip();
            let port = local_addr.port();
            println!("[SERVER] Listening on {ip}:{port}.");
        }

        let router = Arc::clone(&self.router);
        let listener = Arc::clone(&self.listener);
        let do_logging = Arc::clone(&self.do_logging);
        let use_shutdown_route = Arc::clone(&self.use_shutdown_route);
        let keep_listening = Arc::clone(&self.keep_listening);

        keep_listening.store(true, Ordering::Relaxed);

        // Spawn listener thread.
        let handle = spawn(move || {
            // Create a thread pool to handle incoming requests.
            let pool = ThreadPool::new(NUM_WORKER_THREADS);

            while keep_listening.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((reader, mut writer)) => {
                        let do_log = Arc::clone(&do_logging);
                        let inner_router = Arc::clone(&router);
                        let do_keep_listening = Arc::clone(&keep_listening);
                        let do_use_shutdown_rt = Arc::clone(&use_shutdown_route);

                        // Task an available worker thread with responding.
                        pool.execute(move || {
                            let result = Self::handle_connection(
                                reader,
                                &inner_router,
                                &do_log,
                                &do_use_shutdown_rt
                            );

                            match result {
                                Ok(do_shutdown) if do_shutdown => {
                                    do_keep_listening.store(false, Ordering::Relaxed);
                                    writer.send_dummy_request().unwrap();
                                },
                                Err(err1) => {
                                    // Send 500 server error response if there's an error.
                                    let mut res = Response::new(500);

                                    let msg = format!("Error: {}", &err1);
                                    res.body = Body::Text(msg);

                                    res.headers.insert_connection("close");
                                    res.headers.insert_cache_control("no-cache");
                                    res.headers.insert_content_length(res.body.len());
                                    res.headers.insert_content_type("text/plain; charset=utf-8");

                                    match writer.send_response(&mut res) {
                                        Ok(_) if *do_log => {
                                            Self::log_error(&err1);
                                        },
                                        Err(err2) if *do_log => {
                                            Self::log_error(&err1);
                                            Self::log_error(&err2);
                                        },
                                        _ => {},
                                    }
                                },
                                _ => {},
                            }
                        });
                    },
                    Err(e) if *do_logging => Self::log_error(&e),
                    _ => {},
                }
            }
        });

        self.handle = Some(handle);
        Ok(self)
    }

    /// Handles a request from a remote connection.
    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::similar_names)]
    pub fn handle_connection(
        reader: NetReader,
        router: &Arc<Router>,
        do_logging: &Arc<bool>,
        use_shutdown_route: &Arc<bool>
    ) -> NetResult<bool> {
        let mut req = Request::recv(reader)?;
        let route = req.route();

        let mut res = Response::from_route(&route, router);
        res.writer = req.reader
            .take()
            .and_then(|reader| Some(NetWriter::from(reader)));

        if **do_logging {
            let maybe_ip = res.remote_ip();
            let status = res.status_code();
            let method = route.method();
            let path = route.path();

            match maybe_ip {
                Some(ip) => println!("[{ip}|{status}] {method} {path}"),
                None => println!("[?|{status}] {method} {path}"),
            }
        }

        res.send()?;

        // Check for server shutdown signal
        if **use_shutdown_route && route.is_shutdown_route() {
            Ok(true)
        } else {
            Ok(false)
        }
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

    /// Logs a non-terminating server error.
    pub fn log_error(e: &dyn StdError) {
        println!("[SERVER] Error: {e}");
    }

    /// Logs a server shutdown message to stdout.
    pub fn log_error_msg(msg: &str) {
        println!("[SERVER] Error: {msg}");
    }

    /// Logs a server shutdown message to stdout.
    pub fn log_shutdown(&self) {
        println!("[SERVER] Now shutting down.");
    }

    /// Triggers graceful shutdown of the server.
    #[allow(clippy::missing_errors_doc)]
    pub fn shutdown(self) -> NetResult<()> {
        if *self.do_logging {
            self.log_shutdown();
        }

        // Briefly connect to ourselves to unblock the listener thread.
        let _ = self.local_addr()
            .ok_or(IoErrorKind::NotConnected.into())
            .and_then(|addr| TcpStream::connect(addr))
            .and_then(|stream| stream.shutdown(Shutdown::Both))?;

        // Give the worker threads a bit of time to shutdown.
        thread::sleep(Duration::from_millis(200));
        Ok(())
    }
}

pub type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct Worker {
    pub id: usize,
    pub handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Spawns a worker thread that receives tasks and executes them.
    pub fn new(id: usize, receiver: Arc<Mutex<Receiver<Task>>>) -> Self {
        let handle = thread::spawn(move || {
            while let Ok(job) = receiver.lock().unwrap().recv() {
                job();
            }
        });

        Self { id, handle: Some(handle) }
    }
}

/// Holds the pool of `Worker` threads.
pub struct ThreadPool {
    pub workers: Vec<Worker>,
    pub sender: Option<Sender<Task>>,
}

impl ThreadPool {
    /// Create a new `ThreadPool` with the given number of worker threads.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (tx, rx) = channel();
        let sender = Some(tx);
        let receiver = Arc::new(Mutex::new(rx));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        Self { workers, sender }
    }

    /// Sends a `Task` to a worker thread for executon.
    #[allow(clippy::missing_panics_doc)]
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
