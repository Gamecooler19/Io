use std::{
    collections::HashMap,
    hash::Hash,
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;
use futures::{Future, StreamExt};
use async_trait::async_trait;

#[async_trait]
pub trait Loader<K, V> {
    async fn load(&self, keys: &[K]) -> Result<Vec<Option<V>>>;
}

pub struct DataLoader<'ctx, K, V, L>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    L: Loader<K, V> + Send + Sync + 'static,
{
    loader: Arc<L>,
    cache: Arc<Mutex<HashMap<K, V>>>,
    batch_size: usize,
    max_batch_wait: Duration,
    context: &'ctx inkwell::context::Context,
}

impl<'ctx, K, V, L> DataLoader<'ctx, K, V, L>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    L: Loader<K, V> + Send + Sync + 'static,
{
    pub fn new(
        context: &'ctx inkwell::context::Context,
        loader: L,
        batch_size: usize,
        max_batch_wait: Duration,
    ) -> Self {
        Self {
            loader: Arc::new(loader),
            cache: Arc::new(Mutex::new(HashMap::new())),
            batch_size,
            max_batch_wait,
            context,
        }
    }

    pub async fn load(&self, key: K) -> Result<Option<V>> {
        // Check cache first
        if let Some(value) = self.cache.lock().await.get(&key) {
            return Ok(Some(value.clone()));
        }

        // Batch load with other requests
        let values = self.batch_load(vec![key.clone()]).await?;
        Ok(values.into_iter().next().unwrap_or(None))
    }

    pub async fn load_many(&self, keys: Vec<K>) -> Result<Vec<Option<V>>> {
        self.batch_load(keys).await
    }

    async fn batch_load(&self, keys: Vec<K>) -> Result<Vec<Option<V>>> {
        let mut cache = self.cache.lock().await;
        
        // Filter out cached keys
        let missing_keys: Vec<_> = keys.iter()
            .filter(|k| !cache.contains_key(k))
            .cloned()
            .collect();

        if !missing_keys.is_empty() {
            let values = self.loader.load(&missing_keys).await?;
            
            // Update cache with new values
            for (key, value) in missing_keys.into_iter().zip(values.iter()) {
                if let Some(v) = value {
                    cache.insert(key, v.clone());
                }
            }
        }

        // Return values in original order
        Ok(keys.into_iter()
            .map(|k| cache.get(&k).cloned())
            .collect())
    }

    pub fn clear(&self) -> impl Future<Output = ()> {
        let cache = self.cache.clone();
        async move {
            cache.lock().await.clear();
        }
    }

    pub fn clear_key(&self, key: &K) -> impl Future<Output = ()> {
        let cache = self.cache.clone();
        let key = key.clone();
        async move {
            cache.lock().await.remove(&key);
        }
    }
}
