use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::{Result, error::IoError};
use semver::{Version, VersionReq};
use petgraph::{Graph, Directed, graph::NodeIndex};
use petgraph::algo::toposort;
use std::collections::{HashMap, HashSet};

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
        let mut resolution = DependencyResolution::new();
        let mut visited = HashSet::new();
        
        // Build dependency graph
        let mut graph = Graph::<PackageNode, (), Directed>::new();
        let root_idx = graph.add_node(PackageNode {
            name: package.name.clone(),
            version: package.version.clone(),
        });
        
        // Recursively resolve dependencies
        self.resolve_recursive(
            package,
            &mut resolution,
            &mut visited,
            &mut graph,
            root_idx,
            0
        )?;

        // Check for cycles
        if let Err(cycle) = toposort(&graph, None) {
            let cycle_path = self.format_dependency_cycle(&graph, cycle.node_id());
            return Err(IoError::validation_error(
                format!("Circular dependency detected: {}", cycle_path)
            ));
        }

        // Resolve version conflicts
        resolution.resolve_conflicts()?;

        // Convert resolution to ordered list
        Ok(resolution.to_version_list())
    }

    fn resolve_recursive(
        &self,
        package: &Package,
        resolution: &mut DependencyResolution,
        visited: &mut HashSet<String>,
        graph: &mut Graph<PackageNode, ()>,
        parent_idx: NodeIndex,
        depth: usize,
    ) -> Result<()> {
        // Prevent infinite recursion
        if depth > 100 {
            return Err(IoError::validation_error("Maximum dependency depth exceeded"));
        }

        for (dep_name, version_req) in &package.dependencies {
            let key = format!("{}@{}", dep_name, version_req);
            if visited.contains(&key) {
                continue;
            }
            visited.insert(key);

            // Find best matching version
            let best_version = self.find_best_version(dep_name, version_req)?;
            
            // Add to resolution
            resolution.add_dependency(dep_name.clone(), best_version.clone())?;

            // Add to graph
            let child_idx = graph.add_node(PackageNode {
                name: dep_name.clone(),
                version: best_version.version.clone(),
            });
            graph.add_edge(parent_idx, child_idx, ());

            // Resolve transitive dependencies
            if let Some(dep_package) = self.get_package(dep_name, &best_version.version)? {
                self.resolve_recursive(
                    &dep_package,
                    resolution,
                    visited,
                    graph,
                    child_idx,
                    depth + 1
                )?;
            }
        }

        Ok(())
    }

    fn find_best_version(&self, name: &str, version_req: &str) -> Result<&PackageVersion> {
        let req = VersionReq::parse(version_req)
            .map_err(|e| IoError::validation_error(format!("Invalid version requirement: {}", e)))?;

        self.packages
            .get(name)
            .ok_or_else(|| IoError::not_found(format!("Package {} not found", name)))?
            .iter()
            .filter(|v| {
                Version::parse(&v.version)
                    .map(|ver| req.matches(&ver))
                    .unwrap_or(false)
            })
            .max_by(|a, b| {
                Version::parse(&a.version)
                    .unwrap()
                    .cmp(&Version::parse(&b.version).unwrap())
            })
            .ok_or_else(|| IoError::not_found(format!(
                "No matching version found for {} @ {}", name, version_req
            )))
    }

    fn format_dependency_cycle(&self, graph: &Graph<PackageNode, ()>, node: NodeIndex) -> String {
        let mut path = Vec::new();
        let mut current = node;
        
        while let Some(next) = graph.neighbors_directed(current, petgraph::Direction::Incoming).next() {
            path.push(&graph[current].name);
            current = next;
            if current == node {
                break;
            }
        }
        
        path.push(&graph[current].name);
        path.reverse();
        path.join(" -> ")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageVersion {
    version: String,
    dependencies: HashMap<String, String>,
    checksum: String,
}

#[derive(Debug)]
struct DependencyResolution {
    dependencies: HashMap<String, HashSet<PackageVersion>>,
}

impl DependencyResolution {
    fn new() -> Self {
        Self {
            dependencies: HashMap::new(),
        }
    }

    fn add_dependency(&mut self, name: String, version: PackageVersion) -> Result<()> {
        self.dependencies
            .entry(name)
            .or_insert_with(HashSet::new)
            .insert(version);
        Ok(())
    }

    fn resolve_conflicts(&mut self) -> Result<()> {
        for versions in self.dependencies.values_mut() {
            if versions.len() > 1 {
                // Keep only highest compatible version
                let highest = versions
                    .iter()
                    .max_by(|a, b| {
                        Version::parse(&a.version)
                            .unwrap()
                            .cmp(&Version::parse(&b.version).unwrap())
                    })
                    .cloned()
                    .unwrap();
                versions.clear();
                versions.insert(highest);
            }
        }
        Ok(())
    }

    fn to_version_list(&self) -> Vec<PackageVersion> {
        self.dependencies
            .values()
            .flat_map(|versions| versions.iter().cloned())
            .collect()
    }
}

#[derive(Debug, Clone)]
struct PackageNode {
    name: String,
    version: String,
}
