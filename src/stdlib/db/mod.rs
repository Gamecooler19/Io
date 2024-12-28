use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{debug, error, info};
use async_trait::async_trait;

#[derive(Debug)]
pub enum DbError {
    ConnectionError(String),
    QueryError(String),
    DataError(String),
    PoolError(String),
}

impl Error for DbError {}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DbError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            DbError::QueryError(msg) => write!(f, "Query error: {}", msg),
            DbError::DataError(msg) => write!(f, "Data error: {}", msg),
            DbError::PoolError(msg) => write!(f, "Pool error: {}", msg),
        }
    }
}

#[async_trait]
pub trait DbConnection: Send + Sync {
    async fn execute(&self, query: &str) -> Result<Vec<String>, DbError>;
    async fn transaction<F, T>(&self, f: F) -> Result<T, DbError>
    where
        F: FnOnce() -> Result<T, DbError> + Send;
}

pub struct DatabaseConnection {
    connection_string: String,
    pool: Arc<Mutex<Option<Connection>>>,
    max_connections: usize,
    timeout_ms: u64,
}

struct Connection {
    is_connected: bool,
    last_used: std::time::Instant,
    connection_id: String,
}

impl DatabaseConnection {
    pub fn new(connection_string: String, max_connections: usize, timeout_ms: u64) -> Self {
        Self {
            connection_string,
            pool: Arc::new(Mutex::new(None)),
            max_connections,
            timeout_ms,
        }
    }

    pub async fn connect(&self) -> Result<(), DbError> {
        debug!("Establishing database connection");
        let mut pool = self.pool.lock().await;
        
        if pool.is_some() {
            return Err(DbError::ConnectionError("Already connected".to_string()));
        }

        let conn = Connection {
            is_connected: true,
            last_used: std::time::Instant::now(),
            connection_id: uuid::Uuid::new_v4().to_string(),
        };

        *pool = Some(conn);
        info!("Database connection established");
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), DbError> {
        debug!("Disconnecting from database");
        let mut pool = self.pool.lock().await;
        
        if pool.is_none() {
            return Err(DbError::ConnectionError("Not connected".to_string()));
        }

        *pool = None;
        info!("Database disconnected");
        Ok(())
    }

    pub async fn execute_query(&self, query: &str) -> Result<Vec<String>, DbError> {
        let pool = self.pool.lock().await;
        match &*pool {
            Some(conn) => {
                debug!("Executing query with connection {}", conn.connection_id);
                if (!conn.is_connected) {
                    return Err(DbError::ConnectionError("Connection lost".to_string()));
                }
                
                if conn.last_used.elapsed().as_millis() as u64 > self.timeout_ms {
                    return Err(DbError::ConnectionError("Connection timeout".to_string()));
                }
                
                Ok(vec![]) // Simulate query execution
            }
            None => {
                error!("No active database connection");
                Err(DbError::ConnectionError("Not connected".to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_connection() {
        let db = DatabaseConnection::new(
            "test://localhost".to_string(),
            10,
            5000
        );
        
        assert!(db.connect().await.is_ok());
        
        // Test query execution
        let result = db.execute_query("SELECT * FROM test").await;
        assert!(result.is_ok());
        
        // Test disconnection
        assert!(db.disconnect().await.is_ok());
        
        // Test query after disconnection
        let result = db.execute_query("SELECT * FROM test").await;
        assert!(result.is_err());
    }
}
