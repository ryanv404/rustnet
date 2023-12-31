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
    pub fn router(&mut self, router: Router) -> &mut Self {
        self.router = router;
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
        // Mount a shutdown route if this is a test server.
        if self.is_test_server {
            self.router.mount_shutdown_route();
        }

        let mut server = Server {
            do_log: self.do_log,
            do_debug: self.do_debug,
            is_test_server: self.is_test_server,
            router: Arc::new(self.router.clone()),
            listener: None,
            keep_listening: AtomicBool::new(true),
            log_file: None
        };

        match self.listener.take() {
            Some(Ok(listener)) => server.listener = Some(listener),
            Some(Err(e)) => return Err(e),
            None => return Err(NetError::NotConnected),
        }

        if let Some(path) = self.log_file.take() {
            server.do_log = true;
            server.log_file = Some(Arc::new(path));
        }

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
    pub router: Arc<Router>,
    pub listener: Option<Listener>,
    pub keep_listening: AtomicBool,
    pub log_file: Option<Arc<PathBuf>>,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            do_log: false,
            do_debug: false,
            is_test_server: false,
            router: Arc::new(Router::default()),
            listener: None,
            keep_listening: AtomicBool::new(true),
            log_file: None
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
        let mut server = Self::builder();

        let Some(addr) = cli.addr.take() else {
            return Err(NetError::Other("Missing server address.".into()));
        };

        if let Some(path) = cli.log_file.take() {
            let _ = server.log_file(path);
            cli.do_log = true;
        }

        server
            .addr(&addr)
            .do_log(cli.do_log)
            .do_debug(cli.do_debug)
            .router(cli.router.clone())
            .is_test_server(cli.is_test)
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

    /// Logs a server message to stdout or to a local log file.
    pub fn log(&self, msg: &str) {
        if self.do_log {
            let Some(path) = self.log_file.as_ref() else {
                println!("{msg}");
                return;
            };

            let write_res = match OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.as_ref())
            {
                Ok(ref mut fh) => writeln!(fh, "[SERVER] {msg}"),
                Err(ref err) => {
                    eprintln!("[SERVER] {msg}\nLogging error: {err}");
                    return;
                },
            };

            if let Err(ref err) = write_res {
                eprintln!("[SERVER] {msg}\nLogging error: {err}");
            }
        }
    }

    /// Sends a 500 status response to the client if there is an error.
    pub fn send_500_error(
        &self,
        err: &NetError,
        conn: &mut Connection
    ) {
        let msg = format!("[SERVER] Error: {err}");
        self.log(&msg);

        if let Err(ref err2) = conn.send_500_error(&err.to_string()) {
            let msg = format!("[SERVER] Error: {err2}");
            self.log(&msg);
        }
    }

    /// Triggers a graceful shutdown of the local server.
    pub fn shutdown_server(&self, conn: &Connection) {
        let ip = conn.remote_addr.ip();
        let port = conn.remote_addr.port();
        let msg = format!("[SERVER] SHUTDOWN received from {ip}:{port}");
        self.log(&msg);

        self.keep_listening.store(false, Ordering::Relaxed);

        let addr = conn.local_addr;
        let timeout = Duration::from_millis(200);

        // Briefly connect to ourselves to unblock the listener thread.
        if let Err(ref err) = TcpStream::connect_timeout(&addr, timeout)
            .and_then(|stream| stream.shutdown(Shutdown::Both))
        {
            let msg = format!("[SERVER] Error: {err}");
            self.log(&msg);
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

        // Spawn listener thread.
        let handle = spawn(move || {
            let server = Arc::new(self);

            let ip = listener.local_addr.ip();
            let port = listener.local_addr.port();
            let msg = format!("[SERVER] Listening on {ip}:{port}");
            server.log(&msg);

            // Create a thread pool to handle incoming requests.
            let pool = ThreadPool::new(NUM_WORKERS, &server);

            while server.keep_listening.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok(conn) => {
                        // Check if shutdown was triggered.
                        if !server.keep_listening.load(Ordering::Relaxed) {
                            break;
                        }

                        pool.handle_connection(conn);
                    },
                    Err(ref err) => {
                        let msg = format!("[SERVER] Error: {err}");
                        server.log(&msg);
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
