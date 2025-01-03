use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use inkwell::values::PointerValue;

pub struct ConnectionPool<'ctx> {
    connections: Arc<Mutex<VecDeque<Connection<'ctx>>>>,
    max_size: usize,
}

struct Connection<'ctx> {
    handle: PointerValue<'ctx>,
    is_busy: bool,
}

impl<'ctx> ConnectionPool<'ctx> {
    pub fn new(max_size: usize) -> Self {
        Self {
            connections: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }

    pub fn default() -> Self {
        Self::new(10)
    }

    pub fn acquire(&self) -> Option<PooledConnection<'ctx>> {
        let mut conns = self.connections.lock().unwrap();
        conns.iter_mut()
            .find(|c| !c.is_busy)
            .map(|c| {
                c.is_busy = true;
                PooledConnection { 
                    handle: c.handle,
                    pool: Arc::clone(&self.connections)
                }
            })
    }
}

pub struct PooledConnection<'ctx> {
    handle: PointerValue<'ctx>,
    pool: Arc<Mutex<VecDeque<Connection<'ctx>>>>,
}

impl<'ctx> Drop for PooledConnection<'ctx> {
    fn drop(&mut self) {
        if let Ok(mut conns) = self.pool.lock() {
            // Find and mark the connection as not busy
            if let Some(conn) = conns.iter_mut().find(|c| c.handle == self.handle) {
                conn.is_busy = false;
            }
        }
    }
}
