#![forbid(unsafe_code)]

use std::{cell::RefCell, collections::VecDeque, fmt::Debug, rc::Rc};
use thiserror::Error;

////////////////////////////////////////////////////////////////////////////////

// TODO: your code goes here.

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
#[error("channel is closed")]
pub struct SendError<T> {
    pub value: T,
}

pub struct Sender<T> {
    buff: Rc<RefCell<VecDeque<T>>>,
    receiver_closed: Rc<RefCell<bool>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        if self.is_closed() {
            return Err(SendError { value });
        }
        self.buff.borrow_mut().push_back(value);
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        *self.receiver_closed.borrow()
    }

    pub fn same_channel(&self, other: &Self) -> bool {
        self.buff.as_ptr() == other.buff.as_ptr()
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            buff: self.buff.clone(),
            receiver_closed: self.receiver_closed.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // 1 from the receiver and 1 from current sender
        if Rc::strong_count(&self.buff) == 2 {
            *self.receiver_closed.borrow_mut() = true;
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Error, Debug)]
pub enum ReceiveError {
    #[error("channel is empty")]
    Empty,
    #[error("channel is closed")]
    Closed,
}

pub struct Receiver<T> {
    buff: Rc<RefCell<VecDeque<T>>>,
    is_closed: Rc<RefCell<bool>>,
}

impl<T> Receiver<T> {
    pub fn recv(&mut self) -> Result<T, ReceiveError> {
        self.buff.borrow_mut().pop_front().ok_or_else(|| {
            if *self.is_closed.borrow() {
                return ReceiveError::Closed;
            }
            ReceiveError::Empty
        })
    }

    pub fn close(&mut self) {
        *self.is_closed.borrow_mut() = true;
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.close();
    }
}

////////////////////////////////////////////////////////////////////////////////

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let comm_buff = Rc::new(RefCell::new(VecDeque::<T>::new()));
    let comm_closed = Rc::new(RefCell::new(false));
    (
        Sender {
            buff: comm_buff.clone(),
            receiver_closed: comm_closed.clone(),
        },
        Receiver {
            buff: comm_buff,
            is_closed: comm_closed,
        },
    )
}
