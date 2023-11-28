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
    Method, NetError, NetReader, NetResult, Request, Response, Router,
};

/// Configures the socket address and the router for a `Server`.
#[derive(Debug)]
pub struct ServerBuilder<A>
where
    A: ToSocketAddrs
{
    pub ip: Option<IpAddr>,
    pub port: Option<u16>,
    pub addr: Option<A>,
    pub router: Option<Router>,
    pub do_logging: bool,
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
            do_logging: false
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
    pub fn log(mut self, do_logging: bool) -> Self {
        self.do_logging = do_logging;
        self
    }

    /// Builds and returns a `Server` instance.
    pub fn build(mut self) -> NetResult<Server> {
        let router = self.router.take().unwrap_or_default();

        let listener = self.addr
            .as_ref()
            .and_then(|addr| {
                match Listener::bind(addr) {
                    Ok(listener) => Some(listener),
                    Err(_) => match (self.ip, self.port) {
                        (Some(ip), Some(port)) => {
                            Listener::bind_ip_port(ip, port).ok()
                        },
                        (_, _) => None
                    }
                }
            })
            .ok_or_else(|| IoErrorKind::InvalidInput)?;

        Ok(Server {
            router: Arc::new(router),
            listener: Arc::new(listener),
            do_logging: Arc::new(self.do_logging),
            keep_listening: Arc::new(AtomicBool::new(false))
        })
    }

    /// Builds and starts the server.
    pub fn start(self) -> NetResult<()> {
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
    pub fn bind<A>(addr: A) -> NetResult<Self>
    where
        A: ToSocketAddrs
    {
        Ok(Self {
            inner: TcpListener::bind(addr)?
        })
    }

    /// Bind the listener to the given socket address.
    pub fn bind_ip_port(ip: IpAddr, port: u16) -> NetResult<Self> {
        Ok(Self {
            inner: TcpListener::bind((ip, port))?
        })
    }

    /// Returns the server's socket address.
    pub fn local_addr(&self) -> NetResult<SocketAddr> {
        self.inner
            .local_addr()
            .map_err(Into::into)
    }

    /// Returns a `NetReader` instance for each incoming connection.
    pub fn accept(&self) -> NetResult<NetReader> {
        self.inner
            .accept()
            .and_then(|(stream, _)| Ok(NetReader::from(stream)))
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
    /// Trigger for closing the server.
    pub keep_listening: Arc<AtomicBool>,
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
    pub fn new(router: Router, listener: Listener, do_log: bool) -> Self {
        Self {
            router: Arc::new(router),
            listener: Arc::new(listener),
            do_logging: Arc::new(do_log),
            keep_listening: Arc::new(AtomicBool::new(false))
        }
    }

    /// Activates the server to start listening on its bound address.
    pub fn start(self) -> NetResult<()> {
        if *self.do_logging {
            self.log_start_up()?;
        }

        let router = Arc::clone(&self.router);
        let listener = Arc::clone(&self.listener);
        let do_logging = Arc::clone(&self.do_logging);
        let keep_listening = Arc::clone(&self.keep_listening);

        // Spawn listener thread.
        let handle = spawn(move || {
            // Create a thread pool to handle incoming requests.
            let pool = ThreadPool::new(NUM_WORKER_THREADS);

            keep_listening.store(true, Ordering::Relaxed);

            while keep_listening.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(reader) => {
                        let task_rtr = Arc::clone(&router);
                        let do_log = Arc::clone(&do_logging);

                        // Task an available worker thread with responding.
                        pool.execute(move || {
                            match Server::respond(reader, &task_rtr, do_log) {
                                Err(e) => Self::log_error(&e),
                                _ => {},
                            }
                        });
                    },
                    Err(e) => Self::log_error(&e),
                }
            }

            if *do_logging {
                Self::log_shutdown();
            }
        });

        // Wait for the server to finish.
        handle.join().unwrap();

        Ok(())
    }

    /// Handles a request from a remote connection.
    pub fn respond(
        reader: NetReader,
        router: &Arc<Router>,
        do_log: Arc<bool>
    ) -> NetResult<()> {
        let mut req = Request::recv(reader)?;
        let mut res = Response::from_request(&mut req, router)?;

        if *do_log {
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
    pub fn log_start_up(&self) -> NetResult<()> {
        let local_addr = self.listener.local_addr()?;
        let ip = local_addr.ip();
        let port = local_addr.port();
        println!("[SERVER] Listening on {ip}:{port}.");
        Ok(())
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
