use std::net::{
    IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

use crate::{
    Body, NetError, NetReader, NetResult, NetWriter, Response, Route, Router,
    NUM_WORKERS,
};

/// Configures the socket address and the router for a `Server`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    pub is_test: bool,
    pub do_logging: bool,
    pub addr: Option<A>,
    pub router: Router,
}

impl<A> Default for ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    fn default() -> Self {
        Self {
            is_test: false,
            do_logging: false,
            addr: None,
            router: Router::default()
        }
    }
}

impl<A> ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    /// Returns a builder object that is used to build a `Server`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the local address on which the server listens.
    #[must_use]
    pub fn addr(mut self, addr: A) -> Self {
        self.addr = Some(addr);
        self
    }

    /// Adds the given `Router` to the server.
    #[must_use]
    pub fn router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    /// Enable logging of new connections to stdout (default: disabled).
    #[must_use]
    pub const fn is_test(mut self, is_test: bool) -> Self {
        self.is_test = is_test;
        self
    }

    /// Enable logging of new connections to stdout (default: disabled).
    #[must_use]
    pub const fn log_connections(mut self, do_log: bool) -> Self {
        self.do_logging = do_log;
        self
    }

    /// Builds and returns a `Server` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if creating the `Listener` fails.
    pub fn build(self) -> NetResult<Server> {
        let listener = self
            .addr
            .as_ref()
            .ok_or(NetError::NotConnected)
            .and_then(Listener::bind)?;

        let config = ServerConfig {
            is_test: self.is_test,
            do_logging: self.do_logging,
            keep_listening: AtomicBool::new(false),
            router: self.router
        };

        Ok(Server {
            listener: Arc::new(listener),
            config: Arc::new(config),
        })
    }

    /// Builds and starts the server.
    ///
    /// # Errors
    ///
    /// Returns an error if building the `Server` instance fails.
    pub fn start(self) -> NetResult<ServerHandle<()>> {
        self.build().map(Server::start)
    }
}

/// Contains the configuration options for a `Server`.
#[derive(Debug)]
pub struct ServerConfig {
    pub is_test: bool,
    pub do_logging: bool,
    pub keep_listening: AtomicBool,
    pub router: Router,
}

impl ServerConfig {
    /// Logs a message to the terminal on server start up..
    pub fn log_start_up(&self, addr: &SocketAddr) {
        if self.do_logging {
            let ip = addr.ip();
            let port = addr.port();
            println!("[SERVER] Listening on {ip}:{port}");
        }
    }

    /// Logs a message to the terminal on server shutdown.
    pub fn log_shutdown(&self, conn: &Connection) {
        if self.do_logging {
            let ip = conn.remote_ip();
            println!("[SERVER] SHUTDOWN received from {ip}");
        }
    }

    /// Logs a server error to the terminal.
    pub fn log_error(&self, err: &NetError) {
        if self.do_logging {
            println!("[SERVER] Error: {err}");
        }
    }

    /// Logs an incoming request and the response status to the terminal.
    pub fn log_route(&self, status: u16, route: &Route, conn: &Connection) {
        if self.do_logging {
            let ip = conn.remote_ip();
            println!("[{ip}|{status}] {route}");
        }
    }

    /// Sends a 500 status response to the client if there is an error.
    pub fn send_error(&self, writer: &mut NetWriter, err: &NetError) {
        self.log_error(err);

        if let Err(err2) = writer.send_error(&err.to_string()) {
            self.log_error(&err2);
        }
    }

    /// Triggers a graceful shutdown of the local server.
    pub fn shutdown_server(&self, conn: &Connection) {
        self.log_shutdown(conn);

        self.keep_listening.store(false, Ordering::Relaxed);

        // Briefly connect to ourselves to unblock the listener thread.
        let timeout = Duration::from_millis(200);

        match TcpStream::connect_timeout(&conn.local_addr, timeout) {
            Ok(stream) => {
                if let Err(e) = stream.shutdown(Shutdown::Both) {
                    self.log_error(&e.into());
                }
            },
            Err(e) => self.log_error(&e.into()),
        }

        // Give the worker threads some time to shutdown.
        thread::sleep(timeout);
    }
}

/// Represents the TCP connection between a client and a server.
#[derive(Debug)]
pub struct Connection {
    pub reader: NetReader,
    pub writer: NetWriter,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
}

impl TryFrom<TcpStream> for Connection {
    type Error = NetError;

    fn try_from(stream: TcpStream) -> NetResult<Self> {
        let local_addr = stream.local_addr()?;
        let remote_addr = stream.peer_addr()?;
        let reader = NetReader::from(stream.try_clone()?);
        let writer = NetWriter::from(stream);

        Ok(Self {
            reader,
            writer,
            local_addr,
            remote_addr,
        })
    }
}

impl Connection {
    /// Returns the IP address of the remote client.
    #[must_use]
    pub const fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
    }

    /// Returns a clone of this `Connection`.
    ///
    /// # Errors
    ///
    /// An error is returned if cloning of the underlying `NetReader` or
    /// `NetWriter` fails.
    pub fn try_clone(&self) -> NetResult<Self> {
        let local_addr = self.local_addr;
        let remote_addr = self.remote_addr;
        let reader = self.reader.try_clone()?;
        let writer = self.writer.try_clone()?;
        Ok(Self {
            reader,
            writer,
            local_addr,
            remote_addr,
        })
    }
}

/// A wrapper around a `TcpListener` instance.
#[derive(Debug)]
pub struct Listener {
    pub inner: TcpListener,
    pub local_addr: SocketAddr,
}

impl TryFrom<TcpListener> for Listener {
    type Error = NetError;

    fn try_from(inner: TcpListener) -> NetResult<Self> {
        let local_addr = inner.local_addr()?;
        Ok(Self { inner, local_addr })
    }
}

impl Listener {
    /// Bind a `Listener` to a given socket address.
    ///
    /// # Errors
    ///
    /// Returns an error when `TcpListener::bind` returns an error.
    pub fn bind<A>(addr: A) -> NetResult<Self>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr)?;
        Self::try_from(listener)
    }

    /// Returns a `Connection` instance for each incoming connection.
    ///
    /// # Errors
    ///
    /// Returns an error when `TcpStream::try_clone` returns an error.
    pub fn accept(&self) -> NetResult<Connection> {
        self.inner
            .accept()
            .map_err(|err| NetError::Read(err.kind()))
            .and_then(|(stream, remote_addr)| {
                let local_addr = self.local_addr;
                let reader = NetReader::from(stream.try_clone()?);
                let writer = NetWriter::from(stream);

                Ok(Connection {
                    reader,
                    writer,
                    local_addr,
                    remote_addr,
                })
            })
    }
}

/// A handle to the server's listener thread.
#[derive(Debug)]
pub struct ServerHandle<T> {
    pub handle: JoinHandle<T>,
}

impl<T> ServerHandle<T> {
    /// Waits until the server thread is finished.
    ///
    /// # Errors
    ///
    /// An error is returned if the server's listener thread panics.
    pub fn join(self) -> NetResult<T> {
        self.handle
            .join()
            .map_err(|_| NetError::Other("Could not join the server handle."))
    }
}

/// A `Server` contains an active `Listener` and server configurations.
#[derive(Debug)]
pub struct Server {
    pub listener: Arc<Listener>,
    pub config: Arc<ServerConfig>,
}

impl Server {
    /// Returns a `ServerBuilder` instance.
    #[must_use]
    pub fn builder<A>() -> ServerBuilder<A>
    where
        A: ToSocketAddrs,
    {
        ServerBuilder::new()
    }

    /// Returns a `ServerBuilder` object with the address field set.
    #[must_use]
    pub fn http<A>(addr: A) -> ServerBuilder<A>
    where
        A: ToSocketAddrs,
    {
        ServerBuilder::new().addr(addr)
    }

    /// Returns a test `Server` listening on the given address with a route
    /// that is used for graceful shutdown of the server.
    #[must_use]
    pub fn test<A>(addr: A, mut router: Router) -> ServerBuilder<A>
    where
        A: ToSocketAddrs,
    {
        router.mount_shutdown_route();
        ServerBuilder::new().is_test(true).addr(addr).router(router)
    }

    /// Activates the server to start listening on its bound address.
    #[must_use]
    pub fn start(self) -> ServerHandle<()> {
        let config = Arc::clone(&self.config);
        let listener = Arc::clone(&self.listener);

        // Spawn listener thread.
        let handle = spawn(move || {
            config.log_start_up(&listener.local_addr);

            config.keep_listening.store(true, Ordering::Relaxed);

            // Create a thread pool to handle incoming requests.
            let pool = ThreadPool::new(NUM_WORKERS, &config);

            while config.keep_listening.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(conn) => {
                        // Check if shutdown was triggered.
                        if config.keep_listening.load(Ordering::Relaxed) {
                            pool.handle_connection(conn);
                        } else {
                            break;
                        }
                    },
                    Err(ref e) => config.log_error(e),
                }
            }
        });

        ServerHandle { handle }
    }

    /// Returns the local socket address of the server.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.listener.local_addr
    }

    /// Returns the local IP address of the server.
    #[must_use]
    pub fn local_ip(&self) -> IpAddr {
        self.local_addr().ip()
    }

    /// Returns the local port of the server.
    #[must_use]
    pub fn local_port(&self) -> u16 {
        self.local_addr().port()
    }
}

/// Holds a handle to a single worker thread.
#[derive(Debug)]
pub struct Worker {
    pub id: usize,
    pub handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Spawns a worker thread that receives and handles new connections.
    ///
    /// # Panics
    ///
    /// Panics if there is a problem receiving a `Connection`.
    pub fn new(
        id: usize,
        receiver: Arc<Mutex<Receiver<Connection>>>,
        config: Arc<ServerConfig>,
    ) -> Self {
        let handle = thread::spawn(move || {
            let rx = receiver.lock().unwrap();

            while let Ok(mut conn) = rx.recv() {
                let route = match conn.reader.recv_request() {
                    Ok(req) => req.route(),
                    Err(ref err) => {
                        config.send_error(&mut conn.writer, err);
                        continue;
                    },
                };

                let (target, status) = config.router.resolve(&route);

                let mut resp = match Response::from_target(target, status) {
                    Ok(mut resp) => {
                        // Remove body for HEAD requests.
                        if route.is_head() {
                            resp.body = Body::Empty;
                        }

                        resp
                    },
                    Err(ref err) => {
                        config.send_error(&mut conn.writer, err);
                        continue;
                    },
                };

                if let Err(ref err) = conn.writer.send_response(&mut resp) {
                    config.send_error(&mut conn.writer, err);
                    continue;
                }

                // Check for server shutdown signal
                if config.is_test && route.is_shutdown() {
                    config.shutdown_server(&conn);
                    break;
                }

                config.log_route(status, &route, &conn);
            }
        });

        Self {
            id,
            handle: Some(handle),
        }
    }
}

/// Holds the pool of `Worker` threads.
#[derive(Debug)]
pub struct ThreadPool {
    pub workers: Vec<Worker>,
    pub sender: Option<Sender<Connection>>,
}

impl ThreadPool {
    /// Create a new `ThreadPool` with the given number of worker threads.
    ///
    /// # Panics
    ///
    /// Panics if the `size` argument is less than one.
    #[must_use]
    pub fn new(num_workers: usize, config: &Arc<ServerConfig>) -> Self {
        assert!(num_workers > 0);

        let (tx, rx) = channel();
        let sender = Some(tx);
        let receiver = Arc::new(Mutex::new(rx));

        let mut workers = Vec::with_capacity(num_workers);

        for id in 0..num_workers {
            let worker_rx = Arc::clone(&receiver);
            let config_clone = Arc::clone(config);
            let worker = Worker::new(id, worker_rx, config_clone);
            workers.push(worker);
        }

        Self { workers, sender }
    }

    /// Sends a `Connection` to a worker thread for handling.
    ///
    /// # Panics
    ///
    /// Panics if there is a problem sending the `Connection` to the worker
    /// thread.
    pub fn handle_connection(&self, conn: Connection) {
        if let Some(tx) = self.sender.as_ref() {
            tx.send(conn).unwrap();
        }
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
