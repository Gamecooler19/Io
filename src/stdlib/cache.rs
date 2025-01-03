use std::{
    collections::HashMap,
    hash::Hash,
    time::{Duration, Instant},
};

pub struct LruCache<K, V> {
    inner: HashMap<K, (V, Instant)>,
    max_size: usize,
    ttl: Duration,
}

impl<K: Hash + Eq, V> LruCache<K, V> {
    pub fn with_capacity(size: usize) -> Self {
        Self {
            inner: HashMap::with_capacity(size),
            max_size: size,
            ttl: Duration::from_secs(300),
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key).and_then(|(v, time)| {
            if time.elapsed() < self.ttl {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.inner.len() >= self.max_size {
            self.cleanup();
        }
        self.inner.insert(key, (value, Instant::now()));
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        self.inner.retain(|_, (_, time)| time.elapsed() < self.ttl);
    }
}
