use std::{
    collections::HashMap,
    hash::Hash,
    sync::Arc,
    time::{Duration, Instant},
};
use parking_lot::RwLock;
use crate::{Result, error::IoError};

#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    value: V,
    expires_at: Option<Instant>,
    hit_count: u64,
}

pub struct CacheConfig {
    default_ttl: Option<Duration>,
    max_entries: Option<usize>,
    eviction_policy: EvictionPolicy,
}

#[derive(Debug, Clone, Copy)]
pub enum EvictionPolicy {
    LRU,
    LFU,
    FIFO,
}

pub struct Cache<K, V> 
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    config: CacheConfig,
    metrics: Arc<CacheMetrics>,
}

impl<K, V> Cache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    pub fn new(config: CacheConfig) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(CacheMetrics::new()),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write();
        
        if let Some(entry) = entries.get_mut(key) {
            // Check expiration
            if let Some(expires_at) = entry.expires_at {
                if Instant::now() > expires_at {
                    entries.remove(key);
                    self.metrics.increment_miss();
                    return None;
                }
            }

            entry.hit_count += 1;
            self.metrics.increment_hit();
            Some(entry.value.clone())
        } else {
            self.metrics.increment_miss();
            None
        }
    }

    pub fn set(&self, key: K, value: V, ttl: Option<Duration>) -> Result<()> {
        let mut entries = self.entries.write();

        // Check cache size limit
        if let Some(max_entries) = self.config.max_entries {
            if entries.len() >= max_entries {
                self.evict_entries(&mut entries)?;
            }
        }

        let expires_at = ttl.or(self.config.default_ttl)
            .map(|duration| Instant::now() + duration);

        entries.insert(key, CacheEntry {
            value,
            expires_at,
            hit_count: 0,
        });

        self.metrics.increment_set();
        Ok(())
    }

    fn evict_entries(&self, entries: &mut HashMap<K, CacheEntry<V>>) -> Result<()> {
        match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                // Remove least recently used entry
                if let Some(oldest_key) = entries.iter()
                    .min_by_key(|(_, entry)| entry.expires_at)
                    .map(|(k, _)| k.clone())
                {
                    entries.remove(&oldest_key);
                    self.metrics.increment_eviction();
                }
            }
            EvictionPolicy::LFU => {
                // Remove least frequently used entry
                if let Some(least_used_key) = entries.iter()
                    .min_by_key(|(_, entry)| entry.hit_count)
                    .map(|(k, _)| k.clone())
                {
                    entries.remove(&least_used_key);
                    self.metrics.increment_eviction();
                }
            }
            EvictionPolicy::FIFO => {
                // Remove first entry (oldest)
                if let Some(first_key) = entries.keys().next().cloned() {
                    entries.remove(&first_key);
                    self.metrics.increment_eviction();
                }
            }
        }
        Ok(())
    }

    pub fn get_metrics(&self) -> Arc<CacheMetrics> {
        self.metrics.clone()
    }
}

#[derive(Debug, Default)]
pub struct CacheMetrics {
    hits: RwLock<u64>,
    misses: RwLock<u64>,
    sets: RwLock<u64>,
    evictions: RwLock<u64>,
}

impl CacheMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment_hit(&self) {
        *self.hits.write() += 1;
    }

    pub fn increment_miss(&self) {
        *self.misses.write() += 1;
    }

    pub fn increment_set(&self) {
        *self.sets.write() += 1;
    }

    pub fn increment_eviction(&self) {
        *self.evictions.write() += 1;
    }

    pub fn get_stats(&self) -> CacheStats {
        let hits = *self.hits.read();
        let misses = *self.misses.read();
        let total = hits + misses;

        CacheStats {
            hits,
            misses,
            sets: *self.sets.read(),
            evictions: *self.evictions.read(),
            hit_rate: if total > 0 { hits as f64 / total as f64 } else { 0.0 },
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub evictions: u64,
    pub hit_rate: f64,
}
