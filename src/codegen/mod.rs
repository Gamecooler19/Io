pub mod debug;
pub mod llvm;
pub mod passes;
pub mod types;

use crate::{
    ast::{ASTNode, Declaration, Module as AstModule, Parameter},
    error::IoError,
    types::Type,
};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassManager,
    types::BasicTypeEnum,
    values::{BasicValueEnum, FunctionValue},
};
use std::{collections::HashMap, path::Path};

pub trait CodeGenTrait {
    fn initialize(&mut self) -> Result<()>;
    fn generate_module(&mut self, ast: &AstModule) -> Result<()>;
    fn write_output<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    fn generate_module(&mut self, ast: &AstModule) -> Result<()>;
    fn write_output<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    fn optimize(&mut self) -> Result<()>;
    fn verify(&self) -> Result<()>;
}

pub struct CompilerOptions {
    pub opt_level: OptimizationLevel,
    pub debug_info: bool,
    pub target_triple: Option<String>,
    pub cpu_features: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    None,
    Less,
    Default,
    Aggressive,
}

impl Default for CompilerOptions {
    fn default() -> Self {
        Self {
            opt_level: OptimizationLevel::Default,
            debug_info: false,
            target_triple: None,
            cpu_features: None,
        }
    }
}

/// Helper trait for type conversion
pub trait IntoLLVM<'ctx> {
    type Output;
    fn into_llvm(self, context: &'ctx inkwell::context::Context) -> Self::Output;
}

pub struct CodeGeneratorImpl<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    variables: HashMap<String, BasicValueEnum<'ctx>>,
    functions: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> CodeGeneratorImpl<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn generate(&mut self, ast: &ASTNode) -> Result<()> {
        match ast {
            ASTNode::Program(nodes) => {
                for node in nodes {
                    self.generate(node)?;
                }
                Ok(())
            }
            ASTNode::Function {
                name,
                params,
                return_type,
                body,
                is_async,
            } => self.generate_function(name, params, return_type, body, *is_async),
            ASTNode::Statement(stmt) => self.generate_statement(stmt),
            ASTNode::Expression(expr) => {
                self.generate_expression(expr)?;
                Ok(())
            }
            ASTNode::Block(nodes) => {
                for node in nodes {
                    self.generate(node)?;
                }
                Ok(())
            }
            ASTNode::Call { name, args } => {
                self.generate_function_call(name, args)?;
                Ok(())
            }
            ASTNode::If {
                condition,
                then_branch,
                else_branch,
            } => self.generate_if_statement(condition, then_branch, else_branch),

            ASTNode::While { condition, body } => self.generate_while_loop(condition, body),
            ASTNode::Return(expr) => {
                let value = if let Some(expr) = expr {
                    self.generate_expression(expr)?
                } else {
                    self.context.i32_type().const_zero().into()
                };
                self.builder.build_return(Some(&value));
                Ok(())
            }
            ASTNode::Let { name, value } => {
                let value = self.generate_expression(value)?;
                self.variables.insert(name.clone(), value);
                Ok(())
            }
            _ => Err(IoError::runtime_error("Unsupported AST node")),
        }
    }

    fn generate_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_type: &Option<String>,
        body: &[ASTNode],
        is_async: bool,
    ) -> Result<()> {
        let fn_type = self.context.i32_type().fn_type(&[], false);
        let function = self.module.add_function(name, fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        // Generate function body
        for node in body {
            self.generate(node)?;
        }

        // Add return if none exists
        if !self
            .builder
            .get_insert_block()
            .unwrap()
            .get_terminator()
            .is_some()
        {
            self.builder
                .build_return(Some(&self.context.i32_type().const_int(0, false)));
        }

        Ok(())
    }

    fn generate_statement(&mut self, node: &ASTNode) -> Result<()> {
        match node {
            ASTNode::Let { name, value } => {
                let value = self.generate_expression(value)?;
                self.variables.insert(name.clone(), value);
                Ok(())
            }
            ASTNode::Return(expr) => {
                let value = if let Some(expr) = expr {
                    self.generate_expression(expr)?
                } else {
                    self.context.i32_type().const_zero().into()
                };
                self.builder.build_return(Some(&value));
                Ok(())
            }
            ASTNode::If {
                condition,
                then_branch,
                else_branch,
            } => self.generate_if_statement(condition, then_branch, else_branch),
            ASTNode::While { condition, body } => self.generate_while_loop(condition, body),
            _ => Err(IoError::runtime_error("Unsupported statement type")),
        }
    }

    fn generate_expression(&mut self, node: &ASTNode) -> Result<BasicValueEnum<'ctx>> {
        match node {
            ASTNode::Number(value) => Ok(self
                .context
                .i32_type()
                .const_int(*value as u64, false)
                .into()),
            ASTNode::BinaryOp { op, left, right } => {
                let lhs = self.generate_expression(left)?;
                let rhs = self.generate_expression(right)?;

                match op.as_str() {
                    "+" => Ok(self
                        .builder
                        .build_int_add(lhs.into_int_value(), rhs.into_int_value(), "addtmp")
                        .into()),
                    "-" => Ok(self
                        .builder
                        .build_int_sub(lhs.into_int_value(), rhs.into_int_value(), "subtmp")
                        .into()),
                    "*" => Ok(self
                        .builder
                        .build_int_mul(lhs.into_int_value(), rhs.into_int_value(), "multmp")
                        .into()),
                    "/" => Ok(self
                        .builder
                        .build_int_signed_div(lhs.into_int_value(), rhs.into_int_value(), "divtmp")
                        .into()),
                    _ => Err(IoError::runtime_error("Unknown operator")),
                }
            }
            ASTNode::Identifier(name) => self
                .variables
                .get(name)
                .cloned()
                .ok_or_else(|| IoError::runtime_error("Undefined variable")),
            ASTNode::Call { name, args } => self.generate_function_call(name, args),
            _ => Err(IoError::runtime_error("Unsupported expression type")),
        }
    }

    fn generate_if_statement(
        &mut self,
        condition: &ASTNode,
        then_branch: &[ASTNode],
        else_branch: &Option<Vec<ASTNode>>,
    ) -> Result<()> {
        let function = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let cond_value = self.generate_expression(condition)?;

        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "merge");

        self.builder
            .build_conditional_branch(cond_value.into_int_value(), then_block, else_block);

        // Generate then block
        self.builder.position_at_end(then_block);
        for stmt in then_branch {
            self.generate_statement(stmt)?;
        }
        self.builder.build_unconditional_branch(merge_block);

        // Generate else block
        self.builder.position_at_end(else_block);
        if let Some(else_stmts) = else_branch {
            for stmt in else_stmts {
                self.generate_statement(stmt)?;
            }
        }
        self.builder.build_unconditional_branch(merge_block);

        // Continue at merge block
        self.builder.position_at_end(merge_block);
        Ok(())
    }

    fn generate_function_call(
        &mut self,
        name: &str,
        args: &[ASTNode],
    ) -> Result<BasicValueEnum<'ctx>> {
        let function = self
            .module
            .get_function(name)
            .ok_or_else(|| IoError::runtime_error("Unknown function"))?;

        let mut compiled_args = Vec::new();
        for arg in args {
            compiled_args.push(self.generate_expression(arg)?);
        }

        let args_array: Vec<_> = compiled_args.iter().collect();
        Ok(self
            .builder
            .build_call(function, &args_array, "calltmp")
            .try_as_basic_value()
            .left()
            .unwrap())
    }

    pub fn verify_module(&self) -> Result<()> {
        if self.module.verify().is_err() {
            return Err(IoError::runtime_error("Module verification failed"));
        }
        Ok(())
    }

    fn get_llvm_type(&self, ty: &Type) -> Result<BasicTypeEnum<'ctx>> {
        match ty {
            Type::I32 => Ok(self.context.i32_type().into()),
            Type::I64 => Ok(self.context.i64_type().into()),
            Type::F32 => Ok(self.context.f32_type().into()),
            Type::String => Ok(self.context.ptr_type(inkwell::AddressSpace::Generic).into()),
            Type::Array(elem_ty) => {
                let llvm_ty = self.get_llvm_type(elem_ty)?;
                Ok(self.context.array_type(&llvm_ty, 0).into())
            }
            _ => Err(IoError::type_error("Unsupported type")),
        }
    }

    fn verify_debug_info(&self) -> Result<()> {
        if !self.options.debug_info {
            return Ok(());
        }
        // Add debug info verification logic
        if self.module.get_di_compile_unit().is_none() {
            return Err(IoError::codegen_error("No DI compile unit found"));
        }
        Ok(())
    }
}

impl<'ctx> CodeGenerator for LLVMGenerator<'ctx> {
    fn initialize(&mut self) -> Result<()> {
        // Initialize target machine
        let target_triple = inkwell::targets::TargetMachine::get_default_triple();
        inkwell::targets::Target::initialize_all(&inkwell::targets::InitializationConfig::default());

        let target = inkwell::targets::Target::from_triple(&target_triple)
            .map_err(|e| IoError::codegen_error(format!("Failed to get target: {}", e)))?;

        self.target_machine = Some(
            target
                .create_target_machine(
                    &target_triple,
                    "generic",
                    "",
                    OptimizationLevel::Default,
                    inkwell::targets::RelocMode::Default,
                    inkwell::targets::CodeModel::Default,
                )
                .ok_or_else(|| IoError::codegen_error("Failed to create target machine"))?,
        );

        // Initialize pass manager
        self.pass_manager = PassManager::create(&self.module);
        self.pass_manager.add_instruction_combining_pass();
        self.pass_manager.add_reassociate_pass();
        self.pass_manager.add_gvn_pass();
        self.pass_manager.add_cfg_simplification_pass();
        self.pass_manager.initialize();

        Ok(())
    }

    fn generate_module(&mut self, ast: &Module) -> Result<()> {
        // Generate declarations
        for decl in ast.declarations() {
            match decl {
                Declaration::Function(func) => self.generate_function(func)?,
                Declaration::Struct(struct_def) => self.generate_struct(struct_def)?,
                Declaration::Import(import) => self.process_import(import)?,
                Declaration::Global(global) => self.generate_global(global)?,
            }
        }

        // Generate function implementations
        for func in ast.functions() {
            self.generate_function_body(func)?;
        }

        // Generate initialization code
        self.generate_module_init()?;

        Ok(())
    }

    fn write_output<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let target_machine = self
            .target_machine
            .as_ref()
            .ok_or_else(|| IoError::codegen_error("Target machine not initialized"))?;

        // Write object file
        target_machine
            .write_to_file(
                &self.module,
                inkwell::targets::FileType::Object,
                path.as_ref(),
            )
            .map_err(|e| IoError::codegen_error(format!("Failed to write output: {}", e)))?;

        // Write LLVM IR if debug mode is enabled
        if self.options.emit_llvm_ir {
            let ir_path = path.as_ref().with_extension("ll");
            self.module
                .print_to_file(&ir_path)
                .map_err(|e| IoError::codegen_error(format!("Failed to write LLVM IR: {}", e)))?;
        }

        Ok(())
    }

    fn optimize(&mut self) -> Result<()> {
        // Run module-level optimizations
        self.pass_manager.run_on(&self.module);

        // Run function-level optimizations
        for function in self.module.get_functions() {
            // Skip external functions
            if function.is_declaration() {
                continue;
            }

            let func_pass_manager = PassManager::create(&self.module);

            //  Add optimization passes based on optimization level
            match self.options.optimization_level {
                OptimizationLevel::Aggressive => {
                    func_pass_manager.add_instruction_combining_pass();
                    func_pass_manager.add_reassociate_pass();
                    func_pass_manager.add_gvn_pass();
                    func_pass_manager.add_cfg_simplification_pass();
                    func_pass_manager.add_basic_alias_analysis_pass();
                    func_pass_manager.add_promote_memory_to_register_pass();
                    func_pass_manager.add_tail_call_elimination_pass();
                    func_pass_manager.add_ind_var_simplify_pass();
                    func_pass_manager.add_loop_unroll_pass();
                    func_pass_manager.add_loop_vectorize_pass();
                    func_pass_manager.add_dead_code_elimination_pass();
                }
                OptimizationLevel::Default => {
                    func_pass_manager.add_instruction_combining_pass();
                    func_pass_manager.add_reassociate_pass();
                    func_pass_manager.add_gvn_pass();
                    func_pass_manager.add_cfg_simplification_pass();
                    func_pass_manager.add_dead_code_elimination_pass();
                    func_pass_manager.add_correlated_value_propagation_pass();
                }
                OptimizationLevel::Less => {
                    func_pass_manager.add_instruction_combining_pass();
                    func_pass_manager.add_cfg_simplification_pass();
                }
                OptimizationLevel::None => {}
            }

            func_pass_manager.run_on(&function);
        }

        Ok(())
    }

    fn verify(&self) -> Result<()> {
        // Verify module structure
        if let Err(err) = self.module.verify() {
            return Err(IoError::codegen_error(format!(
                "Module verification failed: {}",
                err
            )));
        }

        // Verify each function
        for function in self.module.get_functions() {
            if !function.verify(true) {
                return Err(IoError::codegen_error(format!(
                    "Function verification failed: {}",
                    function.get_name().to_string_lossy()
                )));
            }
        }

        // Verify types
        self.verify_types()?;

        // Verify debug info if enabled
        if self.options.debug_info {
            self.verify_debug_info()?;
        }

        Ok(())
    }
}

// Helper method for type validation
impl<'ctx> LLVMGenerator<'ctx> {
    fn verify_types(&self) -> Result<()> {
        for ty in self.module.get_types() {
            if ty.is_struct_type() {
                let struct_ty = ty.into_struct_type();
                if struct_ty.get_name().is_none() {
                    return Err(IoError::codegen_error("Anonymous struct type found"));
                }
            }
        }
        Ok(())
    }

    fn verify_debug_info(&self) -> Result<()> {
        if !self.options.debug_info {
            return Ok(());
        }
        // Add debug info verification logic
        if self.module.get_debug_metadata_version() == 0 {
            return Err(IoError::codegen_error("No debug metadata found"));
        }
        if self.module.get_di_compile_unit().is_none() {
            return Err(IoError::codegen_error("No DI compile unit found"));
        }
        Ok(())
    }
}

use crate::error::IoError;
use crate::types::Type;
use crate::Result;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, FunctionValue};
use std::collections::HashMap;

pub trait CodeGenerator {
    fn generate_code(&mut self) -> Result<()>;
}

pub struct LLVMCodeGen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    struct_types: HashMap<String, inkwell::types::StructType<'ctx>>,
}

impl<'ctx> LLVMCodeGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
            functions: HashMap::new(),
            struct_types: HashMap::new(),
        }
    }

    pub fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.functions.get(name).copied()
    }

    pub fn add_function(
        &mut self,
        name: &str,
        ret_type: Type,
        param_types: &[Type],
        is_async: bool,
    ) -> Result<FunctionValue<'ctx>> {
        if self.functions.contains_key(name) {
            return Err(IoError::codegen_error(format!(
                "Function {} already defined",
                name
            )));
        }

        let ret_type = ret_type.to_llvm_type(self.context);
        let param_types: Vec<_> = param_types
            .iter()
            .map(|t| t.to_llvm_type(self.context))
            .collect();

        let fn_type = if is_async {
            // Async functions return a future type
            let future_type = self.get_or_create_future_type(&ret_type)?;
            future_type.fn_type(&param_types, false)
        } else {
            ret_type.fn_type(&param_types, false)
        };

        let function = self.module.add_function(name, fn_type, None);
        self.functions.insert(name.to_string(), function);
        Ok(function)
    }

    pub fn add_struct_type(&mut self, name: &str, field_types: &[Type]) -> Result<()> {
        if self.struct_types.contains_key(name) {
            return Err(IoError::codegen_error(format!(
                "Struct {} already defined",
                name
            )));
        }

        let field_types: Vec<_> = field_types
            .iter()
            .map(|t| t.to_llvm_type(self.context))
            .collect();

        let struct_type = self.context.struct_type(&field_types, false);
        self.struct_types.insert(name.to_string(), struct_type);
        Ok(())
    }

    fn get_or_create_future_type(
        &mut self,
        inner_type: &inkwell::types::BasicTypeEnum<'ctx>,
    ) -> Result<inkwell::types::StructType<'ctx>> {
        let name = format!("Future_{}", inner_type.print_to_string());

        if let Some(ty) = self.struct_types.get(&name) {
            return Ok(*ty);
        }

        let future_type = self.context.struct_type(
            &[
                self.context.i8_type().into(), // State
                inner_type.into(),             // Value
            ],
            false,
        );

        self.struct_types.insert(name, future_type);
        Ok(future_type)
    }
}
