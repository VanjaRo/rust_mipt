#![forbid(unsafe_code)]

use crossbeam::channel::{bounded, unbounded, Receiver, Sender};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicU32, AtomicUsize},
        Arc,
    },
    thread,
};

////////////////////////////////////////////////////////////////////////////////

type Job = Box<dyn FnOnce() + Send>;

enum Message {
    NewJob(Job),
    Terminate,
}

pub struct ThreadPool {
    sender: Sender<Message>,
    thread_count: usize,
    busy_thread_count: Arc<AtomicUsize>,
}

impl ThreadPool {
    pub fn new(thread_count: usize) -> Self {
        let (sender, receiver) = unbounded();
        for _ in 0..thread_count {
            let personal_receiver = receiver.clone();
            thread::spawn(move || loop {
                if let Ok(message) = personal_receiver.recv() {
                    match message {
                        Message::NewJob(job) => {
                            let _ = catch_unwind(AssertUnwindSafe(job));
                        }
                        Message::Terminate => break,
                    }
                }
            });
        }
        Self {
            sender,
            thread_count,
            busy_thread_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn spawn<F: Send + 'static, T: Send + 'static>(&self, task: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T,
    {
        let (ret_sender, ret_receiver) = bounded(1);
        let count_cloned = self.busy_thread_count.clone();
        let job = Box::new(move || {
            let res = task();
            let _ = ret_sender.send(res);
            count_cloned.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        });
        let _ = self.sender.send(Message::NewJob(job));
        self.busy_thread_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        JoinHandle {
            receiver: ret_receiver,
        }
    }

    pub fn shutdown(self) {
        loop {
            if self
                .busy_thread_count
                .load(std::sync::atomic::Ordering::Relaxed)
                .eq(&0)
            {
                break;
            }
        }
        for _ in 0..self.thread_count {
            let _ = self.sender.send(Message::Terminate);
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct JoinHandle<T> {
    receiver: Receiver<T>,
}

#[derive(Debug)]
pub struct JoinError {}

impl<T> JoinHandle<T> {
    pub fn join(self) -> Result<T, JoinError> {
        self.receiver.recv().map_err(|_| JoinError {})
    }
}
