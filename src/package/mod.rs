use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    name: String,
    version: String,
    dependencies: HashMap<String, String>,
    entry_point: PathBuf,
}

impl Package {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            dependencies: HashMap::new(),
            entry_point: PathBuf::from("src/main.io"),
        }
    }

    pub fn add_dependency(&mut self, name: &str, version: &str) {
        self.dependencies.insert(name.to_string(), version.to_string());
    }

    pub fn resolve_dependencies(&self) -> Result<DependencyResolution> {
        let mut resolution = DependencyResolution::new();
        self.resolve_recursive(&mut resolution, 0)?;
        resolution.validate()?;
        Ok(resolution)
    }

    fn resolve_recursive(&self, resolution: &mut DependencyResolution, depth: usize) -> Result<()> {
        if depth > resolution.max_depth {
            return Err(IoError::validation_error("Maximum dependency depth exceeded"));
        }

        for (name, version) in &self.dependencies {
            let dep = resolution.add_dependency(name, version)?;
            dep.resolve_recursive(resolution, depth + 1)?;
        }

        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        // Validate package name
        if !is_valid_package_name(&self.name) {
            return Err(IoError::validation_error("Invalid package name"));
        }

        // Validate version
        if !is_valid_version(&self.version) {
            return Err(IoError::validation_error("Invalid version format"));
        }

        // Validate entry point
        if !self.entry_point.exists() {
            return Err(IoError::validation_error("Entry point file not found"));
        }

        // Validate dependencies
        for (name, version) in &self.dependencies {
            if !is_valid_package_name(name) {
                return Err(IoError::validation_error(
                    format!("Invalid dependency name: {}", name)
                ));
            }
            if !is_valid_version(version) {
                return Err(IoError::validation_error(
                    format!("Invalid dependency version: {}", version)
                ));
            }
        }

        Ok(())
    }
}

pub struct DependencyResolution {
    deps: HashMap<String, ResolvedDependency>,
    max_depth: usize,
}

impl DependencyResolution {
    pub fn new() -> Self {
        Self {
            deps: HashMap::new(),
            max_depth: 100,
        }
    }

    pub fn add_dependency(&mut self, name: &str, version: &str) -> Result<&Package> {
        if self.deps.contains_key(name) {
            return Ok(&self.deps[name].package);
        }

        let package = fetch_package(name, version)?;
        self.deps.insert(name.to_string(), ResolvedDependency {
            package,
            version: version.to_string(),
        });

        Ok(&self.deps[name].package)
    }

    pub fn validate(&self) -> Result<()> {
        // Check for version conflicts
        let mut version_map: HashMap<String, HashSet<String>> = HashMap::new();
        
        for (name, dep) in &self.deps {
            version_map.entry(name.clone())
                .or_insert_with(HashSet::new)
                .insert(dep.version.clone());
        }

        for (name, versions) in version_map {
            if versions.len() > 1 {
                return Err(IoError::validation_error(
                    format!("Version conflict for package {}: {:?}", name, versions)
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ResolvedDependency {
    package: Package,
    version: String,
}

fn is_valid_package_name(name: &str) -> bool {
    let name_regex = regex::Regex::new(r"^[a-z][a-z0-9_-]*$").unwrap();
    name_regex.is_match(name)
}

fn is_valid_version(version: &str) -> bool {
    let version_regex = regex::Regex::new(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9]+)?$").unwrap();
    version_regex.is_match(version)
}

fn fetch_package(name: &str, version: &str) -> Result<Package> {
    // TODO: Implement package fetching from registry
    unimplemented!("Package fetching not implemented")
}
