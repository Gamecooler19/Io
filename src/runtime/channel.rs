use std::collections::VecDeque;
use std::sync::{Arc, Mutex, Condvar};
use std::task::Waker;

pub struct Channel<T> {
    inner: Arc<ChannelInner<T>>,
}

struct ChannelInner<T> {
    queue: Mutex<VecDeque<T>>,
    senders: Mutex<usize>,
    receivers: Mutex<usize>,
    senders_waker: Mutex<Option<Waker>>,
    receivers_waker: Mutex<Option<Waker>>,
    condvar: Condvar,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Channel {
            inner: Arc::new(ChannelInner {
                queue: Mutex::new(VecDeque::new()),
                senders: Mutex::new(0),
                receivers: Mutex::new(0),
                senders_waker: Mutex::new(None),
                receivers_waker: Mutex::new(None),
                condvar: Condvar::new(),
            }),
        }
    }

    pub async fn send(&self, value: T) {
        let mut queue = self.inner.queue.lock().unwrap();
        queue.push_back(value);
        self.inner.condvar.notify_one();
    }

    pub async fn recv(&self) -> Option<T> {
        let mut queue = self.inner.queue.lock().unwrap();
        while queue.is_empty() {
            queue = self.inner.condvar.wait(queue).unwrap();
        }
        queue.pop_front()
    }
}
