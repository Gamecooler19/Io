use std::{
    sync::Arc,
    time::Duration,
};
use sqlx::{
    pool::PoolOptions,
    Any, Pool, Transaction,
};
use async_trait::async_trait;
use crate::{Result, error::IoError};

pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

pub struct DatabaseManager {
    pool: Pool<Any>,
    config: DatabaseConfig,
}

impl DatabaseManager {
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        let pool = PoolOptions::<Any>::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_timeout(config.connect_timeout)
            .idle_timeout(config.idle_timeout)
            .max_lifetime(config.max_lifetime)
            .connect(&config.url)
            .await
            .map_err(|e| IoError::runtime_error(format!("Failed to create database pool: {}", e)))?;

        Ok(Self { pool, config })
    }

    pub async fn with_transaction<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&mut Transaction<'_, Any>) -> Result<R>,
    {
        let mut tx = self.pool.begin().await
            .map_err(|e| IoError::runtime_error(format!("Failed to start transaction: {}", e)))?;

        match f(&mut tx).await {
            Ok(result) => {
                tx.commit().await
                    .map_err(|e| IoError::runtime_error(format!("Failed to commit transaction: {}", e)))?;
                Ok(result)
            }
            Err(e) => {
                tx.rollback().await
                    .map_err(|e| IoError::runtime_error(format!("Failed to rollback transaction: {}", e)))?;
                Err(e)
            }
        }
    }

    pub async fn execute(&self, query: &str, params: &[&(dyn sqlx::Encode + Sync)]) -> Result<u64> {
        sqlx::query(query)
            .bind_all(params)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected())
            .map_err(|e| IoError::runtime_error(format!("Failed to execute query: {}", e)))
    }
}

#[async_trait]
pub trait Queryable {
    async fn query(&self, query: &str) -> Result<Vec<HashMap<String, sqlx::Value>>>;
    async fn query_one(&self, query: &str) -> Result<Option<HashMap<String, sqlx::Value>>>;
    async fn execute(&self, query: &str) -> Result<u64>;
}

#[async_trait]
impl Queryable for DatabaseManager {
    async fn query(&self, query: &str) -> Result<Vec<HashMap<String, sqlx::Value>>> {
        sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| {
                        row.columns()
                            .iter()
                            .map(|col| (col.name().to_string(), row.get(col.name())))
                            .collect()
                    })
                    .collect()
            })
            .map_err(|e| IoError::runtime_error(format!("Failed to execute query: {}", e)))
    }

    async fn query_one(&self, query: &str) -> Result<Option<HashMap<String, sqlx::Value>>> {
        sqlx::query(query)
            .fetch_optional(&self.pool)
            .await
            .map(|row| {
                row.map(|r| {
                    r.columns()
                        .iter()
                        .map(|col| (col.name().to_string(), r.get(col.name())))
                        .collect()
                })
            })
            .map_err(|e| IoError::runtime_error(format!("Failed to execute query: {}", e)))
    }

    async fn execute(&self, query: &str) -> Result<u64> {
        sqlx::query(query)
            .execute(&self.pool)
            .await
            .map(|r| r.rows_affected())
            .map_err(|e| IoError::runtime_error(format!("Failed to execute query: {}", e)))
    }
}
