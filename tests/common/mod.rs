use log::debug;
use std::path::PathBuf;

pub struct TestFile {
    pub path: PathBuf,
    pub content: String,
}

impl TestFile {
    pub fn new(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }

    pub fn write_to_disk(&self) -> std::io::Result<()> {
        debug!("Writing test file: {:?}", self.path);
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, &self.content)
    }

    pub fn cleanup(&self) -> std::io::Result<()> {
        debug!("Cleaning up test file: {:?}", self.path);
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

pub fn get_test_file_path(filename: &str) -> PathBuf {
    PathBuf::from("tests/fixtures").join(filename)
}

pub fn create_temp_file(content: &str) -> Result<PathBuf, std::io::Error> {
    let dir = tempfile::tempdir()?;
    let file_path = dir.path().join("test.io");
    std::fs::write(&file_path, content)?;
    Ok(file_path)
}

pub fn create_temp_dir() -> std::io::Result<PathBuf> {
    let temp_dir = std::env::temp_dir().join("callbridge_tests");
    std::fs::create_dir_all(&temp_dir)?;
    Ok(temp_dir)
}

pub fn cleanup_temp_dir(path: &PathBuf) -> std::io::Result<()> {
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_operations() {
        let temp_dir = create_temp_dir().unwrap();
        let test_file = TestFile::new(temp_dir.join("test.txt"), "test content".to_string());

        assert!(test_file.write_to_disk().is_ok());
        assert!(test_file.path.exists());
        assert!(test_file.cleanup().is_ok());
        assert!(!test_file.path.exists());

        cleanup_temp_dir(&temp_dir).unwrap();
    }

    #[test]
    fn test_temp_dir_operations() {
        let temp_dir = create_temp_dir().unwrap();
        assert!(temp_dir.exists());

        cleanup_temp_dir(&temp_dir).unwrap();
        assert!(!temp_dir.exists());
    }
}
