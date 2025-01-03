use std::sync::{Arc, Mutex};
use inkwell::values::PointerValue;

#[derive(Clone)]
struct Connection<'ctx> {
    handle: PointerValue<'ctx>,
    is_open: bool,
}

pub struct ConnectionPool<'ctx> {
    connections: Arc<Mutex<Vec<Connection<'ctx>>>>,
    max_size: usize,
}

impl<'ctx> ConnectionPool<'ctx> {
    pub fn default() -> Self {
        Self::new(10)
    }

    pub fn new(max_size: usize) -> Self {
        Self {
            connections: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            max_size,
        }
    }

    pub fn acquire(&self) -> Option<PooledConnection<'ctx>> {
        let mut conns = self.connections.lock().unwrap();
        conns.iter_mut()
            .find(|conn| !conn.is_open)
            .map(|conn| {
                conn.is_open = true;
                PooledConnection {
                    handle: conn.handle,
                    pool: Arc::downgrade(&self.connections)
                }
            })
    }
}

pub struct PooledConnection<'ctx> {
    handle: PointerValue<'ctx>,
    pool: std::sync::Weak<Mutex<Vec<Connection<'ctx>>>>,
}

impl<'ctx> Drop for PooledConnection<'ctx> {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            let mut conns = pool.lock().unwrap();
            if let Some(conn) = conns.iter_mut().find(|c| c.handle == self.handle) {
                conn.is_open = false;
            }
        }
    }
}
