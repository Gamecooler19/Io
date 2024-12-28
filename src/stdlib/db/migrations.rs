use std::{
    collections::HashMap,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use async_trait::async_trait;
use sqlx::{Pool, Any};
use crate::{Result, error::IoError};

#[derive(Debug)]
pub struct Migration {
    version: i64,
    name: String,
    up: String,
    down: String,
    checksum: String,
}

pub struct MigrationManager {
    pool: Pool<Any>,
    migrations: Vec<Migration>,
    table_name: String,
}

impl MigrationManager {
    pub async fn new(pool: Pool<Any>) -> Result<Self> {
        let manager = Self {
            pool,
            migrations: Vec::new(),
            table_name: "schema_migrations".to_string(),
        };
        manager.init_migration_table().await?;
        Ok(manager)
    }

    async fn init_migration_table(&self) -> Result<()> {
        sqlx::query(&format!(r#"
            CREATE TABLE IF NOT EXISTS {} (
                version BIGINT PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                applied_at TIMESTAMP NOT NULL,
                checksum VARCHAR(64) NOT NULL
            )
        "#, self.table_name))
        .execute(&self.pool)
        .await
        .map_err(|e| IoError::runtime_error(format!("Failed to create migration table: {}", e)))?;

        Ok(())
    }

    pub fn add_migration(&mut self, name: &str, up: &str, down: &str) -> Result<()> {
        let version = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let checksum = sha256::digest(format!("{}{}", up, down));

        self.migrations.push(Migration {
            version,
            name: name.to_string(),
            up: up.to_string(),
            down: down.to_string(),
            checksum,
        });

        Ok(())
    }

    pub async fn migrate(&self) -> Result<()> {
        let mut tx = self.pool.begin().await
            .map_err(|e| IoError::runtime_error(format!("Failed to start transaction: {}", e)))?;

        // Get applied migrations
        let applied: HashMap<i64, String> = sqlx::query(&format!(
            "SELECT version, checksum FROM {}", 
            self.table_name
        ))
        .fetch_all(&mut tx)
        .await
        .map_err(|e| IoError::runtime_error(format!("Failed to fetch migrations: {}", e)))?
        .into_iter()
        .map(|row| (
            row.get::<i64, _>("version"),
            row.get::<String, _>("checksum"),
        ))
        .collect();

        // Apply pending migrations
        for migration in &self.migrations {
            if let Some(checksum) = applied.get(&migration.version) {
                if *checksum != migration.checksum {
                    return Err(IoError::validation_error(format!(
                        "Migration {} checksum mismatch", migration.name
                    )));
                }
                continue;
            }

            // Apply migration
            sqlx::query(&migration.up)
                .execute(&mut tx)
                .await
                .map_err(|e| IoError::runtime_error(format!(
                    "Failed to apply migration {}: {}", 
                    migration.name, e
                )))?;

            // Record migration
            sqlx::query(&format!(
                "INSERT INTO {} (version, name, applied_at, checksum) VALUES ($1, $2, NOW(), $3)",
                self.table_name
            ))
            .bind(migration.version)
            .bind(&migration.name)
            .bind(&migration.checksum)
            .execute(&mut tx)
            .await
            .map_err(|e| IoError::runtime_error(format!(
                "Failed to record migration {}: {}", 
                migration.name, e
            )))?;
        }

        tx.commit().await
            .map_err(|e| IoError::runtime_error(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    pub async fn rollback(&self, steps: u32) -> Result<()> {
        let mut tx = self.pool.begin().await
            .map_err(|e| IoError::runtime_error(format!("Failed to start transaction: {}", e)))?;

        // Get last N applied migrations
        let applied: Vec<(i64, String)> = sqlx::query(&format!(
            "SELECT version, name FROM {} ORDER BY version DESC LIMIT $1",
            self.table_name
        ))
        .bind(steps as i64)
        .fetch_all(&mut tx)
        .await
        .map_err(|e| IoError::runtime_error(format!("Failed to fetch migrations: {}", e)))?
        .into_iter()
        .map(|row| (
            row.get::<i64, _>("version"),
            row.get::<String, _>("name"),
        ))
        .collect();

        // Rollback migrations
        for (version, name) in applied {
            if let Some(migration) = self.migrations.iter().find(|m| m.version == version) {
                sqlx::query(&migration.down)
                    .execute(&mut tx)
                    .await
                    .map_err(|e| IoError::runtime_error(format!(
                        "Failed to rollback migration {}: {}", 
                        name, e
                    )))?;

                sqlx::query(&format!(
                    "DELETE FROM {} WHERE version = $1",
                    self.table_name
                ))
                .bind(version)
                .execute(&mut tx)
                .await
                .map_err(|e| IoError::runtime_error(format!(
                    "Failed to remove migration record {}: {}", 
                    name, e
                )))?;
            }
        }

        tx.commit().await
            .map_err(|e| IoError::runtime_error(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }
}
