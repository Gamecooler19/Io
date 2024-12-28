pub mod debug;
pub mod llvm;
pub mod passes;
pub mod types;

use crate::error::IoError;
use crate::Result;
use std::path::Path;

pub trait CodeGenerator {
    /// Initialize the code generator
    fn initialize(&mut self) -> Result<()>;

    /// Generate code for a module
    fn generate_module(&mut self, ast: &crate::ast::Module) -> Result<()>;

    /// Write output to file
    fn write_output<P: AsRef<Path>>(&self, path: P) -> Result<()>;

    /// Perform optimizations
    fn optimize(&mut self) -> Result<()>;

    /// Verify generated code
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

use crate::{
    ast::{ASTNode, Parameter, Statement},
    types::Type,
};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::BasicTypeEnum,
    values::{BasicValueEnum, FunctionValue},
};
use std::collections::HashMap;

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

    pub fn generate(&mut self, ast: &ASTNode) -> Result<(), IoError> {
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
    ) -> Result<(), IoError> {
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

    fn generate_statement(&mut self, node: &ASTNode) -> Result<(), IoError> {
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

    fn generate_expression(&mut self, node: &ASTNode) -> Result<BasicValueEnum<'ctx>, IoError> {
        match node {
            ASTNode::IntegerLiteral(value) => Ok(self
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
    ) -> Result<(), IoError> {
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
    ) -> Result<BasicValueEnum<'ctx>, IoError> {
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

    pub fn verify_module(&self) -> Result<(), IoError> {
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
            Type::F64 => Ok(self.context.f64_type().into()),
            Type::Bool => Ok(self.context.bool_type().into()),
            Type::String => Ok(self.context.ptr_type(inkwell::AddressSpace::Generic).into()),
            Type::Array(elem_ty) => {
                let llvm_ty = self.get_llvm_type(elem_ty)?;
                Ok(self.context.array_type(&llvm_ty, 0).into())
            }
            _ => Err(IoError::type_error("Unsupported type")),
        }
    }
}

use crate::{ast::Module as AstModule, error::Result};
use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    types::BasicTypeEnum,
    values::{BasicValueEnum, FunctionValue},
};

pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
        }
    }

    pub fn generate(&self, ast: &AstModule) -> Result<()> {
        // Generate LLVM IR for the module
        self.declare_global_functions()?;

        for function in ast.functions() {
            self.generate_function(function)?;
        }

        // Verify the generated module
        if self.module.verify().is_err() {
            return Err("Invalid LLVM module generated".into());
        }

        Ok(())
    }

    fn generate_function(&self, func: &Function) -> Result<FunctionValue<'ctx>> {
        let fn_type = self.get_function_type(func);
        let function = self.module.add_function(&func.name, fn_type, None);

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Generate function body
        self.generate_statements(&func.body)?;

        Ok(function)
    }

    fn get_llvm_type(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        match ty {
            Type::I32 => self.context.i32_type().into(),
            Type::I64 => self.context.i64_type().into(),
            Type::F32 => self.context.f32_type().into(),
            Type::F64 => self.context.f64_type().into(),
            Type::Void => self.context.void_type().into(),
            Type::Bool => self.context.bool_type().into(),
            Type::String => self
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .into(),
            Type::Array(ty) => {
                let llvm_ty = self.get_llvm_type(ty);
                llvm_ty.array_type(0).into()
            }
            Type::Function { params, ret_ty } => {
                let param_types: Vec<_> = params.iter().map(|ty| self.get_llvm_type(ty)).collect();
                let ret_type = self.get_llvm_type(ret_ty);
                ret_type.fn_type(&param_types, false).into()
            }
            Type::Struct { fields } => {
                let field_types: Vec<_> = fields.iter().map(|ty| self.get_llvm_type(ty)).collect();
                self.context.struct_type(&field_types, false).into()
            }
            Type::Pointer(ty) => {
                let llvm_ty = self.get_llvm_type(ty);
                llvm_ty.ptr_type(AddressSpace::Generic).into()
            }
            Type::Unknown => self.context.i8_type().into(),
        }
    }

    fn declare_global_functions(&self) -> Result<()> {
        // Declare standard library functions
        self.declare_print_function()?;
        self.declare_malloc_function()?;
        self.declare_free_function()?;

        // Declare runtime support functions
        self.declare_runtime_functions()?;

        Ok(())
    }

    fn declare_print_function(&self) -> Result<()> {
        let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let print_type = self.context.void_type().fn_type(&[i8_ptr.into()], false);
        self.module.add_function("print", print_type, None);
        Ok(())
    }

    fn declare_runtime_functions(&self) -> Result<()> {
        // Memory management functions
        let void_type = self.context.void_type();
        let i64_type = self.context.i64_type();
        let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::Generic);

        // Runtime initialization
        self.module
            .add_function("rt_init", void_type.fn_type(&[], false), None);

        // Thread local storage
        self.module
            .add_function("rt_get_tls", i8_ptr.fn_type(&[], false), None);

        // Exception handling
        self.declare_exception_functions(i8_ptr, void_type)?;

        // Garbage collection
        self.declare_gc_functions(i8_ptr, i64_type, void_type)?;

        Ok(())
    }

    fn declare_exception_functions(
        &self,
        i8_ptr: PointerType<'ctx>,
        void_type: VoidType<'ctx>,
    ) -> Result<()> {
        // Exception throwing
        self.module
            .add_function("rt_throw", void_type.fn_type(&[i8_ptr.into()], false), None);

        // Try-catch support
        self.module
            .add_function("rt_try_begin", i8_ptr.fn_type(&[], false), None);

        self.module.add_function(
            "rt_try_end",
            void_type.fn_type(&[i8_ptr.into()], false),
            None,
        );

        Ok(())
    }

    fn declare_gc_functions(
        &self,
        i8_ptr: PointerType<'ctx>,
        i64_type: IntType<'ctx>,
        void_type: VoidType<'ctx>,
    ) -> Result<()> {
        // Allocation
        self.module.add_function(
            "rt_gc_alloc",
            i8_ptr.fn_type(&[i64_type.into()], false),
            None,
        );

        // Collection control
        self.module
            .add_function("rt_gc_collect", void_type.fn_type(&[], false), None);

        // Root management
        self.module.add_function(
            "rt_gc_root_push",
            void_type.fn_type(&[i8_ptr.into()], false),
            None,
        );

        self.module
            .add_function("rt_gc_root_pop", void_type.fn_type(&[], false), None);

        Ok(())
    }

    fn generate_statements(&self, statements: &[Statement]) -> Result<()> {
        for stmt in statements {
            match stmt {
                Statement::Expression(expr) => {
                    self.generate_expression(expr)?;
                }
                Statement::Return(expr) => {
                    let value = if let Some(expr) = expr {
                        self.generate_expression(expr)?
                    } else {
                        self.context.void_type().const_zero().into()
                    };
                    self.builder.build_return(Some(&value));
                }
                Statement::Let { name, init, ty } => {
                    let value = self.generate_expression(init)?;
                    let alloca = self.create_local_variable(name, ty.as_ref(), value)?;
                    self.variables.insert(name.clone(), alloca);
                } // Add other statement types...
            }
        }
        Ok(())
    }

    fn create_local_variable(
        &self,
        name: &str,
        ty: Option<&Type>,
        value: BasicValueEnum<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let var_type = if let Some(ty) = ty {
            self.get_llvm_type(ty)
        } else {
            value.get_type()
        };

        let alloca = self.builder.build_alloca(var_type, name);
        self.builder.build_store(alloca, value);
        Ok(alloca)
    }

    fn get_function_type(&self, func: &Function) -> inkwell::types::FunctionType<'ctx> {
        let return_type = match &func.return_type {
            Some(ty) => self.get_llvm_type(ty),
            None => self.context.void_type().into(),
        };

        let param_types: Vec<_> = func
            .parameters
            .iter()
            .map(|param| self.get_llvm_type(&param.r#type)) // Fix: use r#type to escape keyword
            .collect();

        return_type.fn_type(&param_types, false)
    }
}

pub struct LLVMGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
}

impl<'ctx> CodeGenerator for LLVMGenerator<'ctx> {
    fn initialize(&mut self) -> Result<()> {
        // Implementation
        Ok(())
    }

    fn generate_module(&mut self, ast: &crate::ast::Module) -> Result<()> {
        // Implementation
        Ok(())
    }

    fn write_output<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Implementation
        Ok(())
    }

    fn optimize(&mut self) -> Result<()> {
        // Implementation
        Ok(())
    }

    fn verify(&self) -> Result<()> {
        // Implementation
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
