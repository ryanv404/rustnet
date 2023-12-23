use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};
use crate::{Body, Connection, Response, ServerConfig};

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
        receiver: Arc<Mutex<Receiver<Connection>>>,
        config: Arc<ServerConfig>,
    ) -> Self {
        let handle = thread::spawn(move || {
            let rx = receiver.lock().unwrap();

            while let Ok(mut conn) = rx.recv() {
                let route = match conn.recv_request() {
                    Ok(req) => req.route(),
                    Err(ref err) => {
                        config.send_500_error(err, &mut conn);
                        continue;
                    },
                };

                let (target, status) = config.router.resolve(&route);

                let mut resp = match Response::from_target(target, status) {
                    Ok(mut resp) if route.is_head() => {
                        // Remove body for HEAD requests.
                        resp.body = Body::Empty;
                        resp
                    },
                    Ok(resp) => resp,
                    Err(ref err) => {
                        config.send_500_error(err, &mut conn);
                        continue;
                    },
                };

                if let Err(ref err) = conn.send_response(&mut resp) {
                    config.send_500_error(err, &mut conn);
                    continue;
                }

                // Check for server shutdown signal
                if config.is_test_server && route.is_shutdown() {
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
