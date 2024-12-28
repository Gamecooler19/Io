use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::{Result, error::IoError};

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageRegistry {
    packages: HashMap<String, Vec<PackageVersion>>,
    index_path: PathBuf,
}

impl PackageRegistry {
    pub fn new(index_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&index_path)?;
        Ok(Self {
            packages: HashMap::new(),
            index_path,
        })
    }

    pub fn publish_package(&mut self, package: Package) -> Result<()> {
        // Validate package
        self.validate_package(&package)?;
        
        // Add to registry
        self.packages
            .entry(package.name.clone())
            .or_insert_with(Vec::new)
            .push(PackageVersion {
                version: package.version.clone(),
                dependencies: package.dependencies.clone(),
                checksum: self.calculate_checksum(&package)?,
            });

        self.save_index()?;
        Ok(())
    }

    pub fn resolve_dependencies(&self, package: &Package) -> Result<Vec<PackageVersion>> {
        // TODO: Implement dependency resolution
        Ok(Vec::new())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageVersion {
    version: String,
    dependencies: HashMap<String, String>,
    checksum: String,
}
