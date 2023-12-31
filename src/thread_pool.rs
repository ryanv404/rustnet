use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::{Connection, Server};

/// Contains the ID and handle for a single worker thread.
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
        server: Arc<Server>,
        receiver: Arc<Mutex<Receiver<Connection>>>
    ) -> Self {
        let handle = thread::spawn(move || {
            while let Ok(mut conn) = receiver.lock().unwrap().recv() {
                let route = match conn.recv_request() {
                    Ok(req) => req.route(),
                    Err(ref err) => {
                        server.send_500_error(err.to_string(), &mut conn);
                        continue;
                    },
                };

                let mut res = match server.router.resolve(&route) {
                    Ok(res) => res,
                    Err(ref err) => {
                        server.send_500_error(err.to_string(), &mut conn);
                        continue;
                    },
                };

                if let Err(ref err) = conn.send_response(&mut res) {
                    server.send_500_error(err.to_string(), &mut conn);
                    continue;
                }

                // Check for server shutdown signal
                if server.is_test_server && route.is_shutdown() {
                    server.shutdown_server(&conn);
                    break;
                }

                if server.do_log {
                    let ip = conn.remote_addr.ip();
                    let status = res.status_code();
                    server.log(&format!("[{ip}|{status}] {route}"));
                }
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
    pub fn new(num_workers: usize, server: &Arc<Server>) -> Self {
        assert!(num_workers > 0);

        let (tx, rx) = channel();
        let sender = Some(tx);
        let receiver = Arc::new(Mutex::new(rx));

        let mut workers = Vec::with_capacity(num_workers);

        for id in 0..num_workers {
            let server_clone = server.clone();
            let worker_rx = Arc::clone(&receiver);
            let worker = Worker::new(id, server_clone, worker_rx);

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
