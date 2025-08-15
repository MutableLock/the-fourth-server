use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::panic;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {

        if let Some(sender) = &self.sender {
            sender.send(Box::new(job)).unwrap();
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Dropping sender closes the channel and signals workers to stop
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap_or_else(|e| {
                    eprintln!("Worker thread panicked while joining: {:?}", e);
                });
            }
        }
    }
}
struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || {
            loop {
                let message = {
                    let lock = receiver.lock().unwrap();
                    lock.recv()
                };

                match message {
                    Ok(job) => {

                        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                            job();

                        }));

                        if let Err(err) = result {
                            eprintln!("[Worker] Job panicked: {:?}", err);

                        }
                    }
                    Err(_) => {
                        // Channel disconnected, time to shut down
                        break;
                    }
                }
            }

        });

        Worker {
            thread: Some(thread),
        }
    }
}
