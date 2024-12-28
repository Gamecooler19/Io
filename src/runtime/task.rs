use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Task {
    future: Pin<Box<dyn Future<Output = ()> + Send>>,
    state: TaskState,
}

impl Task {
    pub fn new<F>(future: F) -> Self 
    where 
        F: Future<Output = ()> + Send + 'static 
    {
        Self {
            future: Box::pin(future),
            state: TaskState::Ready,
        }
    }

    // ...rest of implementation...
}
