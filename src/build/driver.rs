use crate::{
    codegen::CodeGenerator,
    compiler::Compiler,
    error::{Result, Error},
    package::Package,
};
use log::{info, debug};
use std::sync::Arc;
use std::path::PathBuf;
use inkwell::OptimizationLevel;

pub struct BuildDriver {
    compiler: Compiler,
    output_dir: PathBuf,
    artifacts_dir: PathBuf,
    code_generator: Arc<CodeGenerator>,
    package: Package,
}

impl BuildDriver {
    pub fn new(output_dir: PathBuf) -> Result<Self> {
        let context = inkwell::context::Context::create();
        let compiler = Compiler::new(&context);
        let artifacts_dir = output_dir.join("artifacts");
        
        std::fs::create_dir_all(&artifacts_dir)?;

        Ok(Self {
            compiler,
            output_dir,
            artifacts_dir,
            code_generator: Arc::new(CodeGenerator::new()),
            package: Package::new(),
        })
    }

    pub fn build_project(&mut self, config: BuildConfig) -> Result<()> {
        info!("Starting build with config: {:?}", config);
        
        // Setup build environment
        self.setup_build_environment(&config)?;
        
        // Parse and validate source files
        let parsed_modules = self.parse_source_files(&config.source_files)?;
        
        // Type checking and semantic analysis
        let analyzed_modules = self.analyze_modules(parsed_modules)?;
        
        // Optimization passes
        let optimized_modules = match config.optimization_level {
            OptimizationLevel::None => analyzed_modules,
            _ => self.run_optimizations(analyzed_modules, config.optimization_level)?,
        };

        // Code generation
        let generated_code = self.code_generator.generate(
            optimized_modules,
            config.target,
            config.debug_info,
        )?;

        // Emit output
        self.emit_output(generated_code, &config)?;

        info!("Build completed successfully");
        Ok(())
    }

    fn setup_build_environment(&self, config: &BuildConfig) -> Result<()> {
        // Create necessary directories
        std::fs::create_dir_all(&self.output_dir)?;
        std::fs::create_dir_all(&self.artifacts_dir)?;
        
        // Set up target-specific configuration
        self.compiler.set_target(config.target.clone())?;
        
        // Configure optimization level
        self.compiler.set_optimization_level(config.optimization_level);
        
        // Setup debug information if needed
        if config.debug {
            self.compiler.enable_debug_info();
        }
        
        Ok(())
    }

    fn parse_source_files(&self, sources: &[PathBuf]) -> Result<Vec<ParsedModule>> {
        info!("Parsing {} source files", sources.len());
        sources.iter()
            .map(|path| {
                debug!("Parsing {}", path.display());
                self.compiler.parse_file(path)
            })
            .collect()
    }

    fn analyze_modules(&self, modules: Vec<ParsedModule>) -> Result<Vec<AnalyzedModule>> {
        info!("Running semantic analysis");
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze_modules(modules)
    }

    fn run_optimizations(
        &self,
        modules: Vec<AnalyzedModule>,
        level: OptimizationLevel
    ) -> Result<Vec<OptimizedModule>> {
        info!("Running optimization passes at level {:?}", level);
        let optimizer = Optimizer::new(level);
        optimizer.optimize_modules(modules)
    }

    fn emit_output(&self, generated_code: GeneratedCode, config: &BuildConfig) -> Result<()> {
        let output_path = self.output_dir.join(match config.target {
            Target::Wasm32 => "output.wasm",
            _ => "output",
        });

        info!("Emitting output to {}", output_path.display());
        
        if config.emit_ir {
            let ir_path = output_path.with_extension("ll");
            std::fs::write(ir_path, generated_code.llvm_ir)?;
        }

        generated_code.write_to_file(output_path, config.strip_symbols)?;
        Ok(())
    }

    fn compile_source_file(&mut self, source: &PathBuf) -> Result<()> {
        let source_text = std::fs::read_to_string(source)?;
        
        // Parse source file
        let ast = self.compiler.parse(&source_text)?;
        
        // Perform semantic analysis
        let analyzed = self.compiler.analyze(&ast)?;
        
        // Generate LLVM IR
        let module = self.compiler.generate_ir(&analyzed)?;
        
        // Run optimization passes if needed
        if self.config.optimization_level != OptimizationLevel::None {
            let passes = OptimizationPasses::new(self.config.optimization_level);
            passes.run_on_module(&module)?;
        }
        
        // Write object file
        let object_file = self.artifacts_dir.join(
            source.file_name().unwrap()
        ).with_extension("o");
        
        self.compiler.emit_object(&module, &object_file)?;
        
        Ok(())
    }

    fn link_objects(&self, config: &BuildConfig) -> Result<()> {
        let mut linker = Linker::new();
        
        // Add all object files
        for entry in std::fs::read_dir(&self.artifacts_dir)? {
            let path = entry?.path();
            if path.extension().map_or(false, |ext| ext == "o") {
                linker.add_object(&path)?;
            }
        }
        
        // Add system libraries
        linker.add_system_libs()?;
        
        // Configure target-specific linking
        match config.target {
            Target::Wasm32 => linker.configure_wasm(),
            Target::Native => linker.configure_native(),
            _ => linker.configure_cross_compile(&config.target),
        }?;
        
        // Perform linking
        let output_path = self.output_dir.join(
            if cfg!(target_os = "windows") { "output.exe" } else { "output" }
        );
        
        linker.link(&output_path)?;
        
        Ok(())
    }
}

pub struct BuildConfig {
    pub source_files: Vec<PathBuf>,
    pub target: String,
    pub optimization_level: OptimizationLevel,
    pub debug: bool,
}
