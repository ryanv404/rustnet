use std::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct Worker {
    #[allow(dead_code)]
    id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Task>>>) -> Self {
        let handle = thread::spawn(move || {
            while let Ok(job) = receiver.lock().unwrap().recv() {
                //println!("Worker {id} got a job; executing.");
                job();
            }

            //println!("Worker {id} disconnected; shutting down.");
        });

        Self {
            id,
            handle: Some(handle),
        }
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

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // Send a boxed closure to the channel.
        self.sender.as_ref().unwrap().send(Box::new(f)).unwrap();
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
