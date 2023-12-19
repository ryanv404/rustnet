use std::error::Error as StdError;
use std::net::{
    IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

use crate::{
    NetError, NetReader, NetResult, NetWriter, Response, Router,
    NUM_WORKER_THREADS,
};

/// Configures the socket address and the router for a `Server`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    pub addr: Option<A>,
    pub router: Option<Router>,
    pub do_logging: bool,
    pub has_shutdown_route: bool,
}

impl<A> Default for ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    fn default() -> Self {
        Self {
            addr: None,
            router: None,
            do_logging: false,
            has_shutdown_route: false,
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
    pub const fn do_logging(mut self, do_logging: bool) -> Self {
        self.do_logging = do_logging;
        self
    }

    /// Set whether to add a route to gracefully shutdown the server
    /// (default: disabled).
    #[must_use]
    pub const fn has_shutdown_route(mut self, has_shutdown_route: bool) -> Self {
        self.has_shutdown_route = has_shutdown_route;
        self
    }

    /// Builds and returns a `Server` instance.
    #[allow(clippy::missing_errors_doc)]
    pub fn build(mut self) -> NetResult<Server> {
        let mut router = self.router.take().unwrap_or_default();

        if self.has_shutdown_route {
            router.mount_shutdown_route();
        }

        let listener = self.addr
            .as_ref()
            .ok_or(NetError::NotConnected)
            .and_then(|addr| {
                let listener = Listener::bind(addr)?;
                Ok(Arc::new(listener))
            })?;

        let config = ServerConfig {
            router: Arc::new(router),
            do_logging: Arc::new(self.do_logging),
            has_shutdown_route: Arc::new(self.has_shutdown_route),
            keep_listening: Arc::new(AtomicBool::new(false))
        };

        Ok(Server { listener, config })
    }

    /// Builds and starts the server.
    #[allow(clippy::missing_errors_doc)]
    pub fn start(self) -> NetResult<ServerHandle<()>> {
        let server = self.build()?;
        let handle = server.start()?;
        Ok(handle)
    }
}

#[derive(Debug)]
pub struct Connection {
    pub reader: NetReader,
    pub writer: NetWriter,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
}

impl Connection {
    pub fn remote_ip(&self) -> IpAddr {
        self.remote_addr.ip()
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
    #[allow(clippy::missing_errors_doc)]
    pub fn bind<A>(addr: A) -> NetResult<Self>
    where
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr)?;
        Self::try_from(listener)
    }

    /// Bind a `Listener` to the given IP address and port.
    #[allow(clippy::missing_errors_doc)]
    pub fn bind_ip_port(ip: IpAddr, port: u16) -> NetResult<Self> {
        let listener = TcpListener::bind((ip, port))?;
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
                Ok(Connection { reader, writer, local_addr, remote_addr })
            })
    }
}

/// A handle to the server's listener thread.
#[derive(Debug)]
pub struct ServerHandle<T> {
    pub handle: JoinHandle<T>,
}

impl<T> ServerHandle<T> {
    pub fn join(self) -> NetResult<T> {
        self.handle.join()
            .map_err(|_| NetError::Other("Could not join the server handle."))
    }
}

#[derive(Debug)]
pub struct Server {
    /// The local socket on which the server listens.
    pub listener: Arc<Listener>,
    pub config: ServerConfig
}

impl Server {
    /// Returns a `ServerBuilder` object.
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

    /// Activates the server to start listening on its bound address.
    #[must_use]
    pub fn start(self) -> NetResult<ServerHandle<()>> {
        if *self.config.do_logging {
            println!("[SERVER] Listening on {}", self.local_addr());
        }

        let config = self.config.clone();
        let listener = Arc::clone(&self.listener);

        // Spawn listener thread.
        let handle = spawn(move || {
            let do_logging = Arc::clone(&config.do_logging);
            let keep_listening = Arc::clone(&config.keep_listening);

            config.keep_listening.store(true, Ordering::Relaxed);

            // Create a thread pool to handle incoming requests.
            let pool = ThreadPool::new(NUM_WORKER_THREADS, config);

            while keep_listening.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(conn) => {
                        // Check if shutdown was triggered.
                        if keep_listening.load(Ordering::Relaxed) {
                            pool.handle_connection(conn);
                        } else {
                            break;
                        }
                    }
                    Err(e) if *do_logging => Self::log_error(&e),
                    _ => {}
                }
            }
        });

        Ok(ServerHandle { handle })
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

    /// Logs a non-terminating server error.
    pub fn log_error(e: &dyn StdError) {
        println!("[SERVER] Error: {e}");
    }

    /// Triggers a graceful shutdown of the server.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an error while shutting down the
    /// underlying `TcpStream`.
    pub fn shutdown(server_addr: &SocketAddr) -> NetResult<()> {
        // Briefly connect to ourselves to unblock the listener thread.
        let timeout = Duration::from_millis(200);
        let stream = TcpStream::connect_timeout(server_addr, timeout)?;
        stream.shutdown(Shutdown::Both)?;

        // Give the worker threads some time to shutdown.
        thread::sleep(Duration::from_millis(200));
        Ok(())
    }
}

#[derive(Debug)]
pub struct ServerConfig {
    pub router: Arc<Router>,
    pub do_logging: Arc<bool>,
    pub has_shutdown_route: Arc<bool>,
    pub keep_listening: Arc<AtomicBool>
}

impl Clone for ServerConfig {
    fn clone(&self) -> Self {
        Self {
            router: Arc::clone(&self.router),
            do_logging: Arc::clone(&self.do_logging),
            has_shutdown_route: Arc::clone(&self.has_shutdown_route),
            keep_listening: Arc::clone(&self.keep_listening)
        }
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
        config: ServerConfig,
    ) -> Self {
        let handle = thread::spawn(move || {
            while let Ok(mut conn) = receiver.lock().unwrap().recv() {
                let route = match conn.reader.recv_request() {
                    Ok(req) => req.route(),
                    Err(ref e) => {
                        Self::error_handler(&mut conn.writer, e, &config);
                        continue;
                    },
                };

                let mut resp = match Response::for_route(&route, &config.router) {
                    Ok(resp) => resp,
                    Err(ref e) => {
                        Self::error_handler(&mut conn.writer, e, &config);
                        continue;
                    },
                };

                if let Err(ref e) = conn.writer.send_response(&mut resp) {
                    Self::error_handler(&mut conn.writer, e, &config);
                    continue;
                }

                // Check for server shutdown signal
                if *config.has_shutdown_route && route.is_shutdown() {
                    if *config.do_logging {
                        let ip = conn.remote_ip();
                        println!("[SERVER] SHUTDOWN received from {ip}");
                    }

                    config.keep_listening.store(false, Ordering::Relaxed);

                    if let Err(ref e) = Server::shutdown(&conn.local_addr) {
                        if *config.do_logging {
                            Server::log_error(e);
                        }
                    }

                    continue;
                }

                if *config.do_logging {
                    let ip = conn.remote_ip();
                    let status = resp.status_code();
                    println!("[{ip}|{status}] {route}");
                }
            }
        });

        let handle = Some(handle);

        Self { id, handle }
    }

    /// Sends a 500 status response to the client if there is an error.
    pub fn error_handler(
        writer: &mut NetWriter,
        error: &NetError,
        config: &ServerConfig
    ) {
        if *config.do_logging {
            Server::log_error(&error);
        }

        if let Err(e) = writer.send_server_error(&error.to_string()) {
            if *config.do_logging {
                Server::log_error(&e);
            }
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
    pub fn new(num_workers: usize, config: ServerConfig) -> Self {
        assert!(num_workers > 0);

        let mut workers = Vec::with_capacity(num_workers);

        let (tx, rx) = channel();
        let sender = Some(tx);
        let receiver = Arc::new(Mutex::new(rx));

        for id in 0..num_workers {
            let config = config.clone();
            let receiver = Arc::clone(&receiver);
            let worker = Worker::new(id, receiver, config);
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
