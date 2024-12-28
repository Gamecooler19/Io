use crate::{
    ast::ASTNode,
    error::IoError,
    lexer::Lexer,
    parser::Parser,
    semantic::analyzer::SemanticAnalyzer,
    codegen::llvm::LLVMCodeGen,
    optimizer::Optimizer,
    Result,
};
use inkwell::{
    context::Context,
    module::Module,
    passes::PassManager,
    OptimizationLevel,
};
use std::path::{Path, PathBuf};

pub struct CompilerOptions {
    optimization_level: OptimizationLevel,
    target_triple: Option<String>,
    debug_info: bool,
    lto_enabled: bool,
    metrics_enabled: bool,
}

pub struct Compiler<'ctx> {
    context: &'ctx Context,
    module: Option<Module<'ctx>>,
    options: CompilerOptions,
    metrics: CompilerMetrics,
}

#[derive(Default)]
pub struct CompilerMetrics {
    parse_time: std::time::Duration,
    optimization_time: std::time::Duration,
    codegen_time: std::time::Duration,
    total_nodes: usize,
    optimized_nodes: usize,
}

impl<'ctx> Compiler<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            module: None,
            options: CompilerOptions {
                optimization_level: OptimizationLevel::Default,
                target_triple: None,
                debug_info: true,
                lto_enabled: false,
                metrics_enabled: false,
            },
            metrics: CompilerMetrics::default(),
        }
    }

    pub fn compile(&mut self, input: PathBuf, output: PathBuf) -> Result<()> {
        let start = std::time::Instant::now();

        // Parse source file
        let source = std::fs::read_to_string(&input)?;
        let ast = self.parse_source(&source)?;

        // Optimize AST
        let optimized_ast = self.optimize_ast(ast)?;

        // Generate LLVM IR
        let module = self.generate_ir(&optimized_ast)?;

        // Run optimization passes
        self.run_optimization_passes(&module)?;

        // Generate output
        self.generate_output(&module, &output)?;

        if self.options.metrics_enabled {
            self.metrics.total_nodes = self.count_ast_nodes(&optimized_ast);
            println!("Compilation metrics:\n{}", self.metrics);
        }

        Ok(())
    }

    fn parse_source(&mut self, source: &str) -> Result<ASTNode> {
        let start = std::time::Instant::now();
        let mut parser = Parser::new(source);
        let ast = parser.parse()?;
        self.metrics.parse_time = start.elapsed();
        Ok(ast)
    }

    fn optimize_ast(&mut self, ast: ASTNode) -> Result<ASTNode> {
        let start = std::time::Instant::now();
        
        // Constant folding
        let mut folder = ConstantFolder::new();
        let ast = folder.fold(&ast)?;

        // Constant propagation
        let mut propagator = ConstantPropagator::new();
        let ast = propagator.optimize(&ast)?;

        // Dead code elimination
        let mut eliminator = DeadCodeEliminator::new();
        let ast = eliminator.eliminate(&ast)?;

        self.metrics.optimization_time = start.elapsed();
        self.metrics.optimized_nodes = self.count_ast_nodes(&ast);
        
        Ok(ast)
    }

    fn run_optimization_passes(&self, module: &Module<'ctx>) -> Result<()> {
        let pass_manager = PassManager::create(module);

        // Add optimization passes based on level
        match self.options.optimization_level {
            OptimizationLevel::None => {},
            OptimizationLevel::Less => {
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
                pass_manager.add_gvn_pass();
                pass_manager.add_cfg_simplification_pass();
            },
            OptimizationLevel::Default | OptimizationLevel::Aggressive => {
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
                pass_manager.add_gvn_pass();
                pass_manager.add_cfg_simplification_pass();
                pass_manager.add_basic_alias_analysis_pass();
                pass_manager.add_promote_memory_to_register_pass();
                pass_manager.add_constant_merge_pass();
                pass_manager.add_dead_store_elimination_pass();
                pass_manager.add_aggressive_dce_pass();
                
                if self.options.optimization_level == OptimizationLevel::Aggressive {
                    pass_manager.add_function_inlining_pass();
                    pass_manager.add_global_optimizer_pass();
                    pass_manager.add_ipsccp_pass();
                    pass_manager.add_dead_arg_elimination_pass();
                    pass_manager.add_aggressive_inst_combining_pass();
                    pass_manager.add_tail_call_elimination_pass();
                }
            }
        }

        // Run passes
        pass_manager.initialize();
        
        Ok(())
    }

    fn count_ast_nodes(&self, ast: &ASTNode) -> usize {
        match ast {
            ASTNode::Program(nodes) => {
                1 + nodes.iter().map(|node| self.count_ast_nodes(node)).sum::<usize>()
            }
            ASTNode::Function { body, .. } => {
                1 + body.iter().map(|node| self.count_ast_nodes(node)).sum::<usize>()
            }
            // Add cases for other node types...
            _ => 1,
        }
    }

    // Builder-style configuration methods
    pub fn with_optimization_level(mut self, level: OptimizationLevel) -> Self {
        self.options.optimization_level = level;
        self
    }

    pub fn with_target_triple(mut self, triple: &str) -> Result<Self> {
        self.options.target_triple = Some(triple.to_string());
        Ok(self)
    }

    pub fn with_debug_info(mut self, enabled: bool) -> Self {
        self.options.debug_info = enabled;
        self
    }

    pub fn with_lto(mut self, enabled: bool) -> Self {
        self.options.lto_enabled = enabled;
        self
    }

    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.options.metrics_enabled = enabled;
        self
    }
}

impl std::fmt::Display for CompilerMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Parse time: {:?}", self.parse_time)?;
        writeln!(f, "Optimization time: {:?}", self.optimization_time)?;
        writeln!(f, "Code generation time: {:?}", self.codegen_time)?;
        writeln!(f, "Total AST nodes: {}", self.total_nodes)?;
        writeln!(f, "Nodes after optimization: {}", self.optimized_nodes)?;
        writeln!(f, "Optimization ratio: {:.2}%", 
            (1.0 - (self.optimized_nodes as f64 / self.total_nodes as f64)) * 100.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compile_simple_program() -> Result<()> {
        let context = Context::create();
        let mut compiler = Compiler::new(&context);

        let input = NamedTempFile::new()?;
        let output = NamedTempFile::new()?;

        std::fs::write(input.path(), r#"
            fn main() -> int {
                return 42;
            }
        "#)?;

        compiler.compile(input.path().to_path_buf(), output.path().to_path_buf())?;
        assert!(output.path().exists());

        Ok(())
    }
}
