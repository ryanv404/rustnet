use std::fs::{File, OpenOptions};
use std::io::Write;
use std::net::{
    IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

pub mod cli;
pub use cli::ServerCli;

use crate::{
    Connection, NetError, NetResult, Route, Router, ThreadPool,
};
use crate::config::NUM_WORKERS;

/// Configures the socket address and the router for a `Server`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    pub do_log: bool,
    pub is_test: bool,
    pub addr: Option<A>,
    pub log_file: Option<PathBuf>,
    pub router: Router,
}

impl<A> Default for ServerBuilder<A>
where
    A: ToSocketAddrs,
{
    fn default() -> Self {
        Self {
            do_log: false,
            is_test: false,
            addr: None,
            log_file: None,
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

    /// Enables test server features for this server.
    #[must_use]
    pub const fn test_server(mut self, is_test: bool) -> Self {
        self.is_test = is_test;
        self
    }

    /// Enable logging of new connections to stdout (default: disabled).
    #[must_use]
    pub const fn log(mut self, do_log: bool) -> Self {
        self.do_log = do_log;
        self
    }

    /// Sets a local file to which log messages will be written.
    #[must_use]
    pub fn log_file(mut self, path: Option<PathBuf>) -> Self {
        self.log_file = path;
        self
    }

    /// Builds and returns a `Server` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `TcpListen::bind` fails to bind the provided
    /// address. If logging to a local file is enabled, an error will be
    /// returned if the provided file path is invalid.
    pub fn build(self) -> NetResult<Server> {
        let listener = self
            .addr
            .as_ref()
            .ok_or(NetError::NotConnected)
            .and_then(Listener::bind)?;


        let mut config = ServerConfig {
            do_log: self.do_log,
            is_test: self.is_test,
            keep_listening: AtomicBool::new(true),
            log_file: None,
            router: self.router.clone()
        };

        if let Some(path) = self.log_file.as_ref() {
            let log_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;

            config.do_log = true;
            config.log_file = Some(Arc::new(Mutex::new(log_file)));
        }

        let server = Server {
            listener: Arc::new(listener),
            config: Arc::new(config)
        };

        Ok(server)
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
    pub do_log: bool,
    pub is_test: bool,
    pub keep_listening: AtomicBool,
    pub log_file: Option<Arc<Mutex<File>>>,
    pub router: Router,
}

impl ServerConfig {
    /// Logs a server start up message.
    pub fn log_start_up(&self, addr: &SocketAddr) {
        if self.do_log {
            let ip = addr.ip();
            let port = addr.port();

            if let Some(file) = self.log_file.as_ref() {
                let mut handle = file.lock().unwrap();
                let _ = writeln!(
                    &mut handle,
                    "[SERVER] Listening on {ip}:{port}"
                );
                let _ = handle.flush();
                return;
            }

            println!("[SERVER] Listening on {ip}:{port}");
        }
    }

    /// Logs a server shutdown message.
    pub fn log_shutdown(&self, conn: &Connection) {
        if self.do_log {
            let ip = conn.remote_ip();

            if let Some(file) = self.log_file.as_ref() {
                let mut handle = file.lock().unwrap();
                let _ = writeln!(
                    &mut handle,
                    "[SERVER] SHUTDOWN received from {ip}"
                );
                let _ = handle.flush();
                return;
            }

            println!("[SERVER] SHUTDOWN received from {ip}");
        }
    }

    /// Logs a server error.
    pub fn log_error(&self, err: &NetError) {
        if self.do_log {
            if let Some(file) = self.log_file.as_ref() {
                let mut handle = file.lock().unwrap();
                let _ = writeln!(
                    &mut handle,
                    "[SERVER] Error: {err}"
                );
                let _ = handle.flush();
                return;
            }

            println!("[SERVER] Error: {err}");
        }
    }

    /// Logs an incoming request and the response status.
    pub fn log_route(
        &self,
        status: u16,
        route: &Route,
        conn: &Connection
    ) {
        if self.do_log {
            let ip = conn.remote_ip();

            if let Some(file) = self.log_file.as_ref() {
                let mut handle = file.lock().unwrap();
                let _ = writeln!(
                    &mut handle,
                    "[{ip}|{status}] {route}"
                );
                let _ = handle.flush();
                return;
            }

            println!("[{ip}|{status}] {route}");
        }
    }

    /// Sends a 500 status response to the client if there is an error.
    pub fn send_500_error(
        &self,
        err: &NetError,
        conn: &mut Connection
    ) {
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
                if let Err(e2) = stream.shutdown(Shutdown::Both) {
                    self.log_error(&e2.into());
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
