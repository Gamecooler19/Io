use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use parking_lot::RwLock;
use crate::{error::IoError, Result};

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub exports: HashMap<String, Export>,
    pub imports: HashSet<Import>,
    pub dependencies: Vec<ModuleRef>,
    pub visibility: ModuleVisibility,
}

#[derive(Debug, Clone)]
pub struct Export {
    name: String,
    kind: ExportKind,
    visibility: Visibility,
    documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ExportKind {
    Function(FunctionSignature),
    Type(TypeInfo),
    Value(ValueInfo),
    Module(ModuleRef),
}

#[derive(Debug, Clone)]
pub struct Import {
    module_path: String,
    symbols: Vec<ImportedSymbol>,
    is_wildcard: bool,
    alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleRef {
    name: String,
    version: Option<String>,
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum ModuleVisibility {
    Public,
    Private,
    Internal(HashSet<String>),
}

pub struct ModuleManager {
    modules: Arc<RwLock<HashMap<String, Module>>>,
    module_cache: Arc<RwLock<HashMap<String, ModuleRef>>>,
    root_path: PathBuf,
}

impl ModuleManager {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            module_cache: Arc::new(RwLock::new(HashMap::new())),
            root_path: root_path.as_ref().to_path_buf(),
        }
    }

    pub fn register_module(&self, module: Module) -> Result<()> {
        let mut modules = self.modules.write();
        if modules.contains_key(&module.name) {
            return Err(IoError::validation_error(
                format!("Module {} already exists", module.name)
            ));
        }

        // Validate module path
        if !module.path.starts_with(&self.root_path) {
            return Err(IoError::validation_error(
                format!("Module path must be within root path: {}", self.root_path.display())
            ));
        }

        // Validate and resolve dependencies
        self.validate_dependencies(&module)?;
        
        // Cache module reference
        self.module_cache.write().insert(
            module.name.clone(),
            ModuleRef {
                name: module.name.clone(),
                version: None,
                path: module.path.clone(),
            },
        );

        modules.insert(module.name.clone(), module);
        Ok(())
    }

    pub fn resolve_module(&self, name: &str) -> Result<Arc<Module>> {
        let modules = self.modules.read();
        modules.get(name)
            .cloned()
            .map(Arc::new)
            .ok_or_else(|| IoError::runtime_error(format!("Module {} not found", name)))
    }

    pub fn validate_dependencies(&self, module: &Module) -> Result<()> {
        let mut visited = HashSet::new();
        self.check_circular_dependencies(&module.name, &mut visited, &Vec::new())?;
        
        for dep in &module.dependencies {
            if !self.module_exists(&dep.name) {
                return Err(IoError::validation_error(
                    format!("Dependency {} not found for module {}", dep.name, module.name)
                ));
            }
        }
        Ok(())
    }

    fn check_circular_dependencies(
        &self,
        module_name: &str,
        visited: &mut HashSet<String>,
        path: &Vec<String>,
    ) -> Result<()> {
        if visited.contains(module_name) {
            if path.contains(&module_name.to_string()) {
                let cycle = path.iter()
                    .skip_while(|&n| n != module_name)
                    .chain(std::iter::once(module_name))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                return Err(IoError::validation_error(
                    format!("Circular dependency detected: {}", cycle)
                ));
            }
            return Ok(());
        }

        visited.insert(module_name.to_string());
        let mut new_path = path.clone();
        new_path.push(module_name.to_string());

        if let Some(module) = self.modules.read().get(module_name) {
            for dep in &module.dependencies {
                self.check_circular_dependencies(&dep.name, visited, &new_path)?;
            }
        }

        Ok(())
    }

    pub fn module_exists(&self, name: &str) -> bool {
        self.modules.read().contains_key(name)
    }

    pub fn get_module_exports(&self, name: &str) -> Result<HashMap<String, Export>> {
        self.modules.read()
            .get(name)
            .map(|m| m.exports.clone())
            .ok_or_else(|| IoError::runtime_error(format!("Module {} not found", name)))
    }

    pub fn verify_module_integrity(&self) -> Result<()> {
        let modules = self.modules.read();
        
        // Verify all imports can be resolved
        for module in modules.values() {
            for import in &module.imports {
                if !self.module_exists(&import.module_path) {
                    return Err(IoError::validation_error(format!(
                        "Module {} imports from non-existent module {}",
                        module.name,
                        import.module_path
                    )));
                }

                // Verify imported symbols exist
                if !import.is_wildcard {
                    let exported_symbols = self.get_module_exports(&import.module_path)?;
                    for symbol in &import.symbols {
                        if !exported_symbols.contains_key(&symbol.name) {
                            return Err(IoError::validation_error(format!(
                                "Symbol {} not found in module {}",
                                symbol.name,
                                import.module_path
                            )));
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    pub fn hot_reload(&mut self, module_name: &str, new_source: &str) -> Result<()> {
        let mut parser = Parser::new(new_source);
        let ast = parser.parse()?;
        
        // Create temporary module
        let temp_module = Module {
            name: module_name.to_string(),
            path: self.modules.read().get(module_name)
                .ok_or_else(|| IoError::runtime_error("Module not found"))?
                .path.clone(),
            exports: HashMap::new(),
            imports: HashSet::new(),
            dependencies: Vec::new(),
            visibility: ModuleVisibility::Private,
        };

        // Validate new module
        self.validate_dependencies(&temp_module)?;
        
        // Update module atomically
        let mut modules = self.modules.write();
        if let Some(module) = modules.get_mut(module_name) {
            *module = temp_module;
            Ok(())
        } else {
            Err(IoError::runtime_error("Module not found"))
        }
    }

    pub fn get_dependency_graph(&self) -> Result<DependencyGraph> {
        let modules = self.modules.read();
        let mut graph = DependencyGraph::new();

        for (name, module) in modules.iter() {
            graph.add_node(name.clone());
            for dep in &module.dependencies {
                graph.add_edge(name.clone(), dep.name.clone())?;
            }
        }

        graph.validate()?;
        Ok(graph)
    }

    pub fn analyze_module_metrics(&self, name: &str) -> Result<ModuleMetrics> {
        let modules = self.modules.read();
        let module = modules.get(name)
            .ok_or_else(|| IoError::runtime_error("Module not found"))?;

        Ok(ModuleMetrics {
            export_count: module.exports.len(),
            import_count: module.imports.len(),
            dependency_count: module.dependencies.len(),
            is_cyclic: self.has_cyclic_dependencies(name)?,
            visibility: module.visibility.clone(),
        })
    }

    fn has_cyclic_dependencies(&self, start: &str) -> Result<bool> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        
        fn dfs(
            current: &str,
            modules: &HashMap<String, Module>,
            visited: &mut HashSet<String>,
            stack: &mut HashSet<String>,
        ) -> bool {
            if stack.contains(current) {
                return true;
            }
            if visited.contains(current) {
                return false;
            }

            visited.insert(current.to_string());
            stack.insert(current.to_string());

            if let Some(module) = modules.get(current) {
                for dep in &module.dependencies {
                    if dfs(&dep.name, modules, visited, stack) {
                        return true;
                    }
                }
            }

            stack.remove(current);
            false
        }

        Ok(dfs(start, &self.modules.read(), &mut visited, &mut stack))
    }
}

#[derive(Debug)]
pub struct DependencyGraph {
    nodes: HashSet<String>,
    edges: HashMap<String, HashSet<String>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, name: String) {
        self.nodes.insert(name);
    }

    pub fn add_edge(&mut self, from: String, to: String) -> Result<()> {
        if !self.nodes.contains(&from) || !self.nodes.contains(&to) {
            return Err(IoError::validation_error("Node not found"));
        }
        self.edges.entry(from)
            .or_insert_with(HashSet::new)
            .insert(to);
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        // Check for cycles
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for node in &self.nodes {
            if !visited.contains(node) {
                if self.has_cycle(node, &mut visited, &mut stack) {
                    return Err(IoError::validation_error("Cyclic dependency detected"));
                }
            }
        }
        Ok(())
    }

    fn has_cycle(&self, node: &str, visited: &mut HashSet<String>, stack: &mut HashSet<String>) -> bool {
        if stack.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }

        visited.insert(node.to_string());
        stack.insert(node.to_string());

        if let Some(deps) = self.edges.get(node) {
            for dep in deps {
                if self.has_cycle(dep, visited, stack) {
                    return true;
                }
            }
        }

        stack.remove(node);
        false
    }
}

#[derive(Debug)]
pub struct ModuleMetrics {
    export_count: usize,
    import_count: usize,
    dependency_count: usize,
    is_cyclic: bool,
    visibility: ModuleVisibility,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    params: Vec<(String, String)>, // (name, type)
    return_type: Option<String>,
    is_async: bool,
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    fields: Vec<(String, String)>, // (name, type)
    methods: Vec<FunctionSignature>,
}

#[derive(Debug, Clone)]
pub struct ValueInfo {
    type_name: String,
    is_constant: bool,
}

#[derive(Debug, Clone)]
pub struct ImportedSymbol {
    name: String,
    alias: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Visibility {
    Public,
    Private,
    Protected(Vec<String>),
}
