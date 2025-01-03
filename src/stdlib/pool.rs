use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct ConnectionPool<'ctx> {
    available: Arc<Mutex<VecDeque<Connection<'ctx>>>>,
    max_size: usize,
}

impl<'ctx> ConnectionPool<'ctx> {
    pub fn new(max_size: usize) -> Self {
        Self {
            available: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }

    pub fn default() -> Self {
        Self::new(10)
    }
}

struct Connection<'ctx> {
    id: u64,
    socket: inkwell::values::PointerValue<'ctx>,
}
