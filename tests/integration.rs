pub mod compiler_tests;

use std::path::PathBuf;

pub fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_data")
}

pub fn setup_test_environment() -> PathBuf {
    let test_dir = test_data_dir();
    std::fs::create_dir_all(&test_dir).expect("Failed to create test directory");
    test_dir
}

pub fn cleanup_test_environment(test_dir: &PathBuf) {
    if test_dir.exists() {
        std::fs::remove_dir_all(test_dir).expect("Failed to cleanup test directory");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_setup() {
        let test_dir = setup_test_environment();
        assert!(test_dir.exists());
        cleanup_test_environment(&test_dir);
        assert!(!test_dir.exists());
    }
}
