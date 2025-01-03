use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use reqwest;
use std::fs;
use std::io::Read;
use tempfile::TempDir;
use semver::Version;
use sha2::{Sha256, Digest};

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

#[derive(Debug, Serialize, Deserialize)]
struct RegistryMetadata {
    name: String,
    versions: HashMap<String, VersionMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VersionMetadata {
    sha256: String,
    dependencies: HashMap<String, String>,
    url: String,
}

#[derive(Debug)]
struct Registry {
    url: String,
    cache_dir: PathBuf,
}

impl Registry {
    fn new(url: &str) -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("io-lang")
            .join("packages");
        
        fs::create_dir_all(&cache_dir).unwrap_or_default();
        
        Self {
            url: url.to_string(),
            cache_dir,
        }
    }

    async fn fetch_metadata(&self, package_name: &str) -> Result<RegistryMetadata> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/packages/{}", self.url, package_name);
        
        let response = client.get(&url)
            .send()
            .await
            .map_err(|e| IoError::network_error(format!("Failed to fetch metadata: {}", e)))?;
            
        if !response.status().is_success() {
            return Err(IoError::not_found(format!("Package {} not found", package_name)));
        }

        response.json::<RegistryMetadata>()
            .await
            .map_err(|e| IoError::parse_error(format!("Invalid metadata: {}", e)))
    }

    fn get_cached_package(&self, name: &str, version: &str) -> Option<Package> {
        let cache_path = self.cache_dir
            .join(name)
            .join(version)
            .join("package.json");
            
        if cache_path.exists() {
            if let Ok(contents) = fs::read_to_string(cache_path) {
                return serde_json::from_str(&contents).ok();
            }
        }
        None
    }

    fn cache_package(&self, package: &Package, version: &str) -> Result<()> {
        let package_dir = self.cache_dir
            .join(&package.name)
            .join(version);
            
        fs::create_dir_all(&package_dir)?;
        
        let cache_path = package_dir.join("package.json");
        let contents = serde_json::to_string_pretty(package)?;
        fs::write(cache_path, contents)?;
        
        Ok(())
    }
}

async fn fetch_package(name: &str, version: &str) -> Result<Package> {
    let registry = Registry::new("https://registry.io-lang.org");
    
    // Check cache first
    if let Some(package) = registry.get_cached_package(name, version) {
        return Ok(package);
    }

    // Fetch metadata from registry
    let metadata = registry.fetch_metadata(name).await?;
    
    // Find matching version
    let version_req = semver::VersionReq::parse(version)
        .map_err(|e| IoError::validation_error(format!("Invalid version requirement: {}", e)))?;
        
    let version_metadata = metadata.versions.iter()
        .filter(|(v, _)| {
            Version::parse(v).map_or(false, |ver| version_req.matches(&ver))
        })
        .max_by(|(a, _), (b, _)| Version::parse(a).unwrap().cmp(&Version::parse(b).unwrap()))
        .ok_or_else(|| IoError::not_found(format!("No matching version found for {}", version)))?;

    // Download package
    let client = reqwest::Client::new();
    let response = client.get(&version_metadata.1.url)
        .send()
        .await
        .map_err(|e| IoError::network_error(format!("Failed to download package: {}", e)))?;

    // Verify checksum
    let mut hasher = Sha256::new();
    let bytes = response.bytes()
        .await
        .map_err(|e| IoError::network_error(format!("Failed to read package data: {}", e)))?;
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());

    if hash != version_metadata.1.sha256 {
        return Err(IoError::validation_error("Package checksum mismatch"));
    }

    // Extract package
    let temp_dir = TempDir::new()?;
    let archive_path = temp_dir.path().join("package.tar.gz");
    fs::write(&archive_path, bytes)?;
    
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(
        fs::File::open(archive_path)?
    ));
    archive.unpack(temp_dir.path())?;

    // Load package manifest
    let manifest_path = temp_dir.path().join("package.json");
    let package: Package = serde_json::from_reader(fs::File::open(manifest_path)?)?;

    // Cache the package
    registry.cache_package(&package, version_metadata.0)?;

    Ok(package)
}

// Add new error variants
impl IoError {
    fn network_error(msg: String) -> Self {
        IoError::RuntimeError(format!("Network error: {}", msg))
    }

    fn parse_error(msg: String) -> Self {
        IoError::RuntimeError(format!("Parse error: {}", msg))
    }

    fn not_found(msg: String) -> Self {
        IoError::RuntimeError(format!("Not found: {}", msg))
    }
}
