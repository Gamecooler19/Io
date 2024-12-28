mod common;
mod integration;
mod unit;

pub(crate) mod test_utils {
    use log::{debug, info};
    use std::env;
    use std::path::PathBuf;
    use std::sync::Once;

    static INIT: Once = Once::new();
    const TEST_DATA_DIR: &str = "test_data";

    pub struct TestContext {
        pub data_dir: PathBuf,
        pub config: TestConfig,
    }

    pub struct TestConfig {
        pub db_url: String,
        pub timeout_ms: u64,
    }

    pub fn setup_test_env() -> TestContext {
        INIT.call_once(|| {
            env::set_var("RUST_LOG", "debug");
            env_logger::init();
            info!("Test environment initialized");
        });

        let data_dir = PathBuf::from(TEST_DATA_DIR);
        std::fs::create_dir_all(&data_dir).expect("Failed to create test directory");

        TestContext {
            data_dir,
            config: TestConfig {
                db_url: "sqlite://test.db".to_string(),
                timeout_ms: 5000,
            },
        }
    }

    pub fn cleanup_test_env(ctx: &TestContext) {
        debug!("Cleaning up test environment");
        if ctx.data_dir.exists() {
            std::fs::remove_dir_all(&ctx.data_dir).expect("Failed to cleanup test directory");
        }
    }

    pub async fn create_test_db_connection(
        config: &TestConfig,
    ) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Creating test database connection");
        // Simulate database connection setup
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    pub fn generate_test_data(size: usize) -> Vec<String> {
        (0..size).map(|i| format!("test_data_{}", i)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_cleanup() {
        let ctx = test_utils::setup_test_env();
        assert!(ctx.data_dir.exists());

        let result = test_utils::create_test_db_connection(&ctx.config).await;
        assert!(result.is_ok());

        let test_data = test_utils::generate_test_data(5);
        assert_eq!(test_data.len(), 5);

        test_utils::cleanup_test_env(&ctx);
        assert!(!ctx.data_dir.exists());
    }
}
