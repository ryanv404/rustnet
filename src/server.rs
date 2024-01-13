use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::fs::OpenOptions;
use std::io::Write;
use std::net::{
    Shutdown, SocketAddr, TcpListener, TcpStream, ToSocketAddrs,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, spawn, JoinHandle};
use std::time::Duration;

pub mod cli;
pub use cli::ServerCli;

use crate::{Connection, NetError, NetResult, Router, ThreadPool};

pub const NUM_WORKERS: usize = 4;

/// Configures the socket address and the router for a `Server`.
#[derive(Debug, Default)]
pub struct ServerBuilder {
    pub do_log: bool,
    pub do_debug: bool,
    pub is_test_server: bool,
    pub listener: Option<NetResult<Listener>>,
    pub router: Router,
    pub log_file: Option<PathBuf>,
}

impl ServerBuilder {
    /// Returns a builder object that is used to build a `Server`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the local address on which the server listens.
    #[must_use]
    pub fn addr<A: ToSocketAddrs>(&mut self, addr: A) -> &mut Self {
        self.listener = Some(Listener::bind(addr));
        self
    }

    /// Adds the given `Router` to the server.
    #[must_use]
    pub fn router(&mut self, router: &mut Router) -> &mut Self {
        self.router.append(router);
        self
    }

    /// Enable debug printing.
    #[must_use]
    pub fn do_debug(&mut self, do_debug: bool) -> &mut Self {
        self.do_debug = do_debug;
        self
    }

    /// Enable logging of new connections to stdout (default: disabled).
    #[must_use]
    pub fn do_log(&mut self, do_log: bool) -> &mut Self {
        self.do_log = do_log;
        self
    }

    /// Sets a local file to which log messages will be written.
    #[must_use]
    pub fn log_file<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.log_file = Some(path.into());
        self
    }

    /// Enables test server features for this server.
    #[must_use]
    pub fn is_test_server(&mut self, is_test: bool) -> &mut Self {
        self.is_test_server = is_test;
        self
    }

    /// Builds and returns a `Server` instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `TcpListen::bind` fails to bind the provided
    /// address. If logging to a local file is enabled, an error will be
    /// returned if the provided file path is invalid.
    pub fn build(&mut self) -> NetResult<Server> {
        if self.is_test_server {
            // Mount a shutdown route if this is a test server.
            let _ = self.router.shutdown();
        }

        let log_file = self.log_file.take().and_then(|path| {
            self.do_log = true;
            Some(Arc::new(path))
        });

        let listener = match self.listener.take() {
            Some(Ok(listener)) => Some(listener),
            Some(Err(e)) => return Err(e),
            None => None,
        };

        let server = Server {
            do_log: self.do_log,
            do_debug: self.do_debug,
            is_test_server: self.is_test_server,
            keep_listening: AtomicBool::new(false),
            listener,
            log_file,
            router: Arc::new(self.router.clone())
        };

        Ok(server)
    }

    /// Builds and starts the server.
    ///
    /// # Errors
    ///
    /// Returns an error if building the `Server` instance fails.
    pub fn start(&mut self) -> NetResult<ServerHandle<()>> {
        let server = self.build()?;
        server.start()
    }
}

/// A `Server` contains an active `Listener` and the server configuration.
#[derive(Debug)]
pub struct Server {
    pub do_log: bool,
    pub do_debug: bool,
    pub is_test_server: bool,
    pub keep_listening: AtomicBool,
    pub listener: Option<Listener>,
    pub log_file: Option<Arc<PathBuf>>,
    pub router: Arc<Router>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            do_log: false,
            do_debug: false,
            is_test_server: false,
            keep_listening: AtomicBool::new(false),
            listener: None,
            log_file: None,
            router: Arc::new(Router::default())
        }
    }
}

impl PartialEq for Server {
    fn eq(&self, other: &Self) -> bool {
        let keep_listening1 = self.keep_listening.load(Ordering::Relaxed);
        let keep_listening2 = other.keep_listening.load(Ordering::Relaxed);

        self.do_log == other.do_log
            && self.do_debug == other.do_debug
            && self.is_test_server == other.is_test_server
            && self.router == other.router
            && self.listener.is_some() == other.listener.is_some()
            && keep_listening1 == keep_listening2
            && self.log_file == other.log_file
    }
}

impl Eq for Server {}

impl TryFrom<ServerCli> for Server {
    type Error = NetError;

    fn try_from(mut cli: ServerCli) -> NetResult<Self> {
        let Some(addr) = cli.addr.take() else {
            return Err(NetError::Other("Missing server address.".into()));
        };

        let mut server = Self::builder();

        if let Some(path) = cli.log_file.take() {
            let _ = server.log_file(path);
            cli.do_log = true;
        }

        server
            .addr(&addr)
            .do_log(cli.do_log)
            .do_debug(cli.do_debug)
            .is_test_server(cli.is_test)
            .router(&mut cli.router)
            .build()
    }
}

impl Server {
    /// Returns a `ServerBuilder` instance.
    #[must_use]
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    /// Returns a `ServerBuilder` object with the address field set.
    #[must_use]
    pub fn http<A: ToSocketAddrs>(addr: A) -> ServerBuilder {
        let mut builder = ServerBuilder::new();
        let _ = builder.addr(addr);
        builder
    }

    /// Logs a server message to the terminal or to a log file.
    pub fn log(&self, msg: &str) {
        if self.do_log {
            let Some(path) = self.log_file.as_ref() else {
                // If no log file is set, then write the log message to stdout.
                return println!("{msg}");
            };

            match OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.as_ref())
            {
                // Write the log message to the log file...
                Ok(ref mut f) => { let _ = writeln!(f, "{msg}"); },
                // ...or to stderr if the log file cannot be opened.
                Err(ref err) => eprintln!("{msg}\nLogging error: {err}"),
            }
        }
    }

    /// Writes a status 500 server error response to the given `Connection`.
    pub fn send_500_error(&self, err: String, conn: &mut Connection) {
        self.log(&format!("[SERVER] Error: {}", &err));

        if let Err(ref err2) = conn.send_500_error(err) {
            self.log(&format!("[SERVER] Error: {err2}"));
        }
    }

    /// Returns true if this `Server` is listening for a new `Connection`.
    pub fn do_listen(&self) -> bool {
        self.keep_listening.load(Ordering::Relaxed)
    }

    /// Returns true if this `Server` is shutting down.
    pub fn do_shutdown(&self) -> bool {
        !self.do_listen()
    }

    /// Triggers a graceful shutdown of the server.
    pub fn shutdown(&self, conn: &Connection) {
        let ip = conn.remote_addr.ip();
        self.log(&format!("[SERVER] SHUTDOWN received from {ip}"));

        self.keep_listening.store(false, Ordering::Relaxed);

        let addr = conn.local_addr;
        let timeout = Duration::from_millis(200);

        // Briefly connect to ourselves to unblock the listener thread.
        if let Err(ref err) = TcpStream::connect_timeout(&addr, timeout)
            .and_then(|stream| stream.shutdown(Shutdown::Both))
        {
            self.log(&format!("[SERVER] Error: {err}"));
        }

        // Give the worker threads some time to shutdown.
        thread::sleep(timeout);
    }

    /// Activates the server to begin listening on its bound address.
    ///
    /// # Errors
    ///
    /// Returns an error if the `Listener` is not active.
    pub fn start(mut self) -> NetResult<ServerHandle<()>> {
        let listener = self.listener.take().ok_or(NetError::NotConnected)?;

        self.keep_listening.store(true, Ordering::Relaxed);

        let server = Arc::new(self);

        // Spawn listener thread.
        let handle = spawn(move || {
            let addr = listener.local_addr;
            server.log(&format!("[SERVER] Listening on {addr}"));

            // Create a thread pool of workers to handle incoming requests.
            let pool = ThreadPool::new(NUM_WORKERS, &server);

            while server.do_listen() {
                match listener.accept() {
                    // Since this thread blocks on `accept`, check if server
                    // shutdown has been triggered.
                    Ok(_) if server.do_shutdown() => break,
                    Ok(conn) => pool.handle_connection(conn),
                    Err(ref err) => {
                        server.log(&format!("[SERVER] Error: {err}"));
                    },
                }
            }
        });

        Ok(ServerHandle { handle })
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
pub struct Listener {
    pub inner: TcpListener,
    pub local_addr: SocketAddr,
}

impl Display for Listener {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:?}")
    }
}

impl Debug for Listener {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Listener {{ ... }}")
    }
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
