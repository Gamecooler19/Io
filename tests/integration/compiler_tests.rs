use crate::test_utils::{cleanup_test_env, setup_test_env, TestContext};
use log::{debug, error};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug)]
enum CompilationTarget {
    Binary,
    Library,
    Object,
}

#[derive(Debug)]
struct CompilerOptions {
    target: CompilationTarget,
    optimization_level: u8,
    debug_info: bool,
    extra_flags: Vec<String>,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            target: CompilationTarget::Binary,
            optimization_level: 0,
            debug_info: true,
            extra_flags: Vec::new(),
        }
    }
}

struct CompilerTestContext {
    test_ctx: TestContext,
    output_dir: PathBuf,
    source_files: Vec<PathBuf>,
    compiler_options: CompilerOptions,
    dependencies: HashMap<String, String>,
}

impl CompilerTestContext {
    fn new() -> Self {
        let test_ctx = setup_test_env();
        let output_dir = test_ctx.data_dir.join("compiler_output");
        std::fs::create_dir_all(&output_dir).expect("Failed to create compiler output directory");

        Self {
            test_ctx,
            output_dir,
            source_files: Vec::new(),
            compiler_options: CompilerOptions::default(),
            dependencies: HashMap::new(),
        }
    }

    fn with_options(mut self, options: CompilerOptions) -> Self {
        self.compiler_options = options;
        self
    }

    fn add_dependency(&mut self, name: &str, version: &str) {
        self.dependencies
            .insert(name.to_string(), version.to_string());
    }

    fn cleanup(self) {
        cleanup_test_env(&self.test_ctx);
    }

    fn create_test_file(&mut self, name: &str, content: &str) -> PathBuf {
        let file_path = self.test_ctx.data_dir.join(name);
        std::fs::write(&file_path, content).expect("Failed to write test file");
        self.source_files.push(file_path.clone());
        file_path
    }

    fn compile_source(&self, source_file: &PathBuf) -> Result<PathBuf, String> {
        debug!("Compiling source file: {:?}", source_file);

        if !source_file.exists() {
            return Err("Source file does not exist".to_string());
        }

        let output_file = self.output_dir.join(
            source_file
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(".cb", ".out"),
        );

        // Simulate compilation process
        std::fs::write(&output_file, "compiled_content").map_err(|e| e.to_string())?;

        Ok(output_file)
    }

    fn compile_project(&self) -> Result<Vec<PathBuf>, String> {
        debug!(
            "Compiling project with {} source files",
            self.source_files.len()
        );

        let mut outputs = Vec::new();
        for source_file in &self.source_files {
            match self.compile_source(source_file) {
                Ok(output) => outputs.push(output),
                Err(e) => {
                    error!("Failed to compile {}: {}", source_file.display(), e);
                    return Err(format!("Compilation failed: {}", e));
                }
            }
        }

        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_compilation() {
        let mut ctx = CompilerTestContext::new();

        let source = r#"
            fn main() {
                print("Hello, World!");
            }
        "#;

        let source_file = ctx.create_test_file("basic.cb", source);
        assert!(source_file.exists());

        let result = ctx.compile_source(&source_file);
        assert!(result.is_ok());
        assert!(result.unwrap().exists());

        ctx.cleanup();
    }

    #[test]
    fn test_compilation_with_options() {
        let options = CompilerOptions {
            target: CompilationTarget::Library,
            optimization_level: 2,
            debug_info: false,
            extra_flags: vec!["--no-std".to_string()],
        };

        let mut ctx = CompilerTestContext::new().with_options(options);

        let source = r#"
            pub fn add(a: int, b: int) -> int {
                return a + b;
            }
        "#;

        let source_file = ctx.create_test_file("lib.cb", source);
        let result = ctx.compile_source(&source_file);
        assert!(result.is_ok());

        ctx.cleanup();
    }

    #[test]
    fn test_multiple_files() {
        let mut ctx = CompilerTestContext::new();

        let main_source = r#"
            fn main() {
                let result = utils::calculate(5);
                print(result);
            }
        "#;

        let utils_source = r#"
            mod utils {
                fn calculate(x: int) -> int {
                    return x * 2;
                }
            }
        "#;

        ctx.create_test_file("main.cb", main_source);
        ctx.create_test_file("utils.cb", utils_source);

        let result = ctx.compile_project();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);

        ctx.cleanup();
    }

    #[test]
    fn test_compilation_with_dependencies() {
        let mut ctx = CompilerTestContext::new();
        ctx.add_dependency("std", "1.0.0");
        ctx.add_dependency("math", "0.1.0");

        let source = r#"
            use std::io;
            use math::complex;

            fn main() {
                let c = complex::new(1.0, 2.0);
                io::println(c.to_string());
            }
        "#;

        let source_file = ctx.create_test_file("deps.cb", source);
        let result = ctx.compile_source(&source_file);
        assert!(result.is_ok());

        ctx.cleanup();
    }

    #[test]
    fn test_compilation_error_handling() {
        let mut ctx = CompilerTestContext::new();

        let invalid_source = r#"
            fn main() {
                let x: int = "not an integer";  // Type mismatch error
                let y = undefined_variable;      // Undefined variable error
                return "wrong return type";      // Return type error
            }
        "#;

        let source_file = ctx.create_test_file("invalid.cb", invalid_source);
        let result = ctx.compile_source(&source_file);
        assert!(result.is_err());

        ctx.cleanup();
    }
}
