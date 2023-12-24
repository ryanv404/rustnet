use std::net::{
    IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

use crate::{
    Connection, NetError, NetResult, Route, Router, ThreadPool, NUM_WORKERS,
};

/// Configures the socket address and the router for a `Server`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    pub addr: Option<A>,
    pub do_logging: bool,
    pub is_test_server: bool,
    pub router: Router,
}

impl<A> Default for ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    fn default() -> Self {
        Self {
            addr: None,
            do_logging: false,
            is_test_server: false,
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
    pub fn router(mut self, router: &Router) -> Self {
        self.router = router.clone();
        self
    }

    /// Enable logging of new connections to stdout (default: disabled).
    #[must_use]
    pub const fn is_test_server(mut self, is_test: bool) -> Self {
        self.is_test_server = is_test;
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
            do_logging: self.do_logging,
            is_test_server: self.is_test_server,
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
    pub do_logging: bool,
    pub is_test_server: bool,
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
    pub fn send_500_error(&self, err: &NetError, conn: &mut Connection) {
        self.log_error(err);

        if let Err(err2) = conn.send_500_error(&err.to_string()) {
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
            Err(e) => self.log_error(&e.into()),
            Ok(stream) => {
                if let Err(e) = stream.shutdown(Shutdown::Both) {
                    self.log_error(&e.into());
                }
            },
        }

        // Give the worker threads some time to shutdown.
        thread::sleep(timeout);
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
                    Err(ref e) => config.log_error(e),
                    Ok(conn) => {
                        // Check if shutdown was triggered.
                        if config.keep_listening.load(Ordering::Relaxed) {
                            pool.handle_connection(conn);
                        } else {
                            break;
                        }
                    },
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
        self.handle.join().map_err(|_| NetError::JoinFailure)
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
    /// Bind a TCP `Listener` to a given address. This function accepts
    /// any input that implements the `ToSocketAddrs` trait.
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
    /// Returns an error if `Connection::try_clone` fails.
    pub fn accept(&self) -> NetResult<Connection> {
        self.inner
            .accept()
            .map_err(|err| NetError::Read(err.kind()))
            .and_then(|(stream, remote_addr)| {
                Connection::try_from((stream, remote_addr))
            })
    }
}
