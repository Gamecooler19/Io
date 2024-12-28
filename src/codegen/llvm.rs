use crate::{
    ast::{ASTNode, ASTVisitor, BinaryOperator, Parameter},
    error::IoError,
    types::Type,
    Result,
};
use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    debug_info::{
        DIFile, DIFlags, DIScope, DISubprogram, DWARFEmissionKind, DWARFSourceLanguage,
        DebugInfoBuilder,
    },
    module::Module,
    passes::PassManager,
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum},
    values::{BasicValue, BasicValueEnum, FunctionValue},
    AddressSpace, OptimizationLevel,
};
use std::collections::HashMap;

pub struct LLVMCodeGen<'ctx> {
    pub(crate) context: &'ctx Context,
    pub(crate) module: Module<'ctx>,
    pub(crate) builder: Builder<'ctx>,
    named_values: HashMap<String, BasicValueEnum<'ctx>>,
    current_function: Option<FunctionValue<'ctx>>,
    optimization_level: OptimizationLevel,
    debug_info: DebugInfo<'ctx>,
    function_pass_manager: inkwell::passes::PassManager<FunctionValue<'ctx>>,
    types: HashMap<String, BasicTypeEnum<'ctx>>,
}

impl<'ctx> LLVMCodeGen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let (debug_builder, _) = module.create_debug_info_builder(
            true,
            DWARFSourceLanguage::C,
            "source.cb",
            ".",
            "CallBridge",
            false,
            "",
            0,
            "",
            DWARFEmissionKind::Full,
            0,
            false,
            false,
            "",
            "",
        );

        let function_pass_manager = PassManager::create(&module);

        Self {
            context,
            module,
            builder,
            named_values: HashMap::new(),
            current_function: None,
            optimization_level: OptimizationLevel::Default,
            debug_info,
            function_pass_manager,
            types: HashMap::new(),
        }
    }

    pub fn generate(&mut self, node: &ASTNode) -> Result<()> {
        self.visit_node(node)?;
        if self.module.verify().is_err() {
            return Err(IoError::runtime_error("LLVM module verification failed"));
        }
        Ok(())
    }

    fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.module.get_function(name)
    }

    fn create_entry_block_alloca(
        &self,
        function: FunctionValue<'ctx>,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let builder = self.context.create_builder();
        let entry = function.get_first_basic_block().unwrap();
        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry),
        }
        inkwell::values::BasicValueEnum::PointerValue(builder.build_alloca(ty, name))
    }

    fn generate_binary_op(
        &self,
        op: &BinaryOperator,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        match (left.get_type(), right.get_type()) {
            (l, r) if l.is_int_type() && r.is_int_type() => {
                let l = left.into_int_value();
                let r = right.into_int_value();
                Ok(match op {
                    BinaryOperator::Add => self.builder.build_int_add(l, r, "addtmp").into(),
                    BinaryOperator::Subtract => self.builder.build_int_sub(l, r, "subtmp").into(),
                    BinaryOperator::Multiply => self.builder.build_int_mul(l, r, "multmp").into(),
                    BinaryOperator::Divide => {
                        self.builder.build_int_signed_div(l, r, "divtmp").into()
                    }
                    // Add more operators...
                    _ => return Err(IoError::runtime_error("Unsupported binary operator")),
                })
            }
            // Add support for other types (float, etc.)
            _ => Err(IoError::runtime_error(
                "Invalid operand types for binary operator",
            )),
        }
    }

    fn add_debug_info(&mut self, node: &ASTNode, scope: DIScope<'ctx>) -> Result<()> {
        if let Some(location) = node.location() {
            let debug_loc = self.debug_info.create_debug_location(
                self.context,
                location.line as u32,
                location.column as u32,
                scope,
                None,
            );
            self.builder.set_current_debug_location(debug_loc);
        }
        Ok(())
    }

    fn create_debug_function(
        &self,
        name: &str,
        linkage_name: Option<&str>,
        file: DIFile<'ctx>,
        line: u32,
    ) -> DISubprogram<'ctx> {
        self.debug_info.create_function(
            scope,
            name,
            linkage_name,
            file,
            line,
            self.debug_info
                .create_subroutine_type(file, None, &[], DIFlags::Zero),
            true,
            true,
            line,
            DIFlags::Zero,
            false,
        )
    }

    fn optimize_function(&mut self, function: FunctionValue<'ctx>) -> Result<()> {
        // Basic optimizations
        self.function_pass_manager.add_instruction_combining_pass();
        self.function_pass_manager.add_reassociate_pass();
        self.function_pass_manager.add_gvn_pass();
        self.function_pass_manager.add_cfg_simplification_pass();
        self.function_pass_manager.add_basic_alias_analysis_pass();
        self.function_pass_manager
            .add_promote_memory_to_register_pass();

        // Advanced optimizations
        if self.optimization_level >= OptimizationLevel::Aggressive {
            self.function_pass_manager.add_tail_call_elimination_pass();
            self.function_pass_manager.add_memcpy_optimization_pass();
            self.function_pass_manager.add_dead_store_elimination_pass();
            self.function_pass_manager.add_licm_pass();
            self.function_pass_manager.add_loop_unroll_pass();
            self.function_pass_manager.add_loop_vectorize_pass();
            self.function_pass_manager.add_slp_vectorize_pass();
        }

        self.function_pass_manager.run_on(&function);
        Ok(())
    }

    fn generate_debug_info(
        &self,
        function: FunctionValue<'ctx>,
        source_location: &SourceLocation,
    ) -> Result<()> {
        let file = self
            .debug_builder
            .create_file(&source_location.file, &source_location.directory);

        let scope = self.debug_builder.create_function(
            file,
            &function.get_name().to_string_lossy(),
            None,
            file,
            source_location.line,
            false,
            true,
            source_location.line,
            0,
            false,
        );

        self.debug_builder.set_current_debug_location(
            source_location.line,
            source_location.column,
            scope,
            None,
        );

        Ok(())
    }

    fn create_intrinsic(&self, name: &str) -> Result<FunctionValue<'ctx>> {
        let intrinsic = match name {
            "memcpy" => self.module.add_function(
                "llvm.memcpy.p0i8.p0i8.i64",
                self.context.void_type().fn_type(
                    &[
                        self.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .into(),
                        self.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .into(),
                        self.context.i64_type().into(),
                        self.context.bool_type().into(),
                    ],
                    false,
                ),
                None,
            ),
            // Add more intrinsics...
            _ => {
                return Err(IoError::runtime_error(format!(
                    "Unknown intrinsic: {}",
                    name
                )))
            }
        };

        Ok(intrinsic)
    }

    // Add helper methods for type creation
    pub fn i8_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i8_type()
    }

    pub fn i32_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i32_type()
    }

    pub fn i64_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i64_type()
    }

    pub fn f64_type(&self) -> inkwell::types::FloatType<'ctx> {
        self.context.f64_type()
    }

    pub fn bool_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.bool_type()
    }

    pub fn void_type(&self) -> inkwell::types::VoidType<'ctx> {
        self.context.void_type()
    }

    pub fn string_type(&self) -> inkwell::types::PointerType<'ctx> {
        self.i8_type().ptr_type(Default::default())
    }

    pub fn register_type(&mut self, name: &str, ty: BasicTypeEnum<'ctx>) -> Result<()> {
        self.types.insert(name.to_string(), ty);
        Ok(())
    }

    pub fn get_type(&self, name: &str) -> Result<BasicTypeEnum<'ctx>> {
        self.types
            .get(name)
            .copied()
            .ok_or_else(|| crate::error::IoError::type_error(format!("Type {} not found", name)))
    }

    fn get_llvm_type(&self, type_name: &str) -> Result<BasicTypeEnum<'ctx>> {
        match type_name {
            "i8" => Ok(self.i8_type().as_basic_type_enum()),
            "i32" => Ok(self.i32_type().as_basic_type_enum()),
            "i64" => Ok(self.i64_type().as_basic_type_enum()),
            "f64" => Ok(self.f64_type().as_basic_type_enum()),
            "bool" => Ok(self.bool_type().as_basic_type_enum()),
            "void" => Err(IoError::type_error(
                "void type cannot be used as a basic type",
            )),
            _ => self.get_type(type_name),
        }
    }

    fn optimize(&mut self) {
        if matches!(self.optimization_level, OptimizationLevel::Aggressive) {
            self.function_pass_manager.add_memcpy_optimize_pass();
        }
    }

    fn create_debug_info(&self, scope: DIScope<'ctx>) -> Result<()> {
        let flags = DIFlags::Zero;
        self.debug_info
            .create_subroutine_type(scope, None, &[], flags);
        Ok(())
    }

    // Add conversion helpers
    fn convert_to_metadata_types(
        &self,
        types: &[BasicTypeEnum<'ctx>],
    ) -> Vec<BasicMetadataTypeEnum<'ctx>> {
        types.iter().map(|t| t.into()).collect()
    }

    pub fn add_external_function(
        &mut self,
        name: &str,
        fn_type: BasicTypeEnum<'ctx>,
    ) -> Result<FunctionValue<'ctx>> {
        let function = self
            .module
            .add_function(name, fn_type.into_function_type(), None);
        Ok(function)
    }

    fn create_function(
        &mut self,
        ret_type: BasicTypeEnum<'ctx>,
        param_types: &[BasicMetadataTypeEnum<'ctx>],
    ) -> Result<()> {
        let fn_type = ret_type.fn_type(param_types, false);
        // ...existing code...
        Ok(())
    }
}

impl<'ctx> ASTVisitor for LLVMCodeGen<'ctx> {
    type Output = BasicValueEnum<'ctx>;

    fn visit_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_type: &Option<String>,
        body: &[ASTNode],
        is_async: bool,
    ) -> Result<Self::Output> {
        let ret_type = self.get_llvm_type(return_type.as_deref().unwrap_or("unit"))?;
        let param_types: Vec<_> = params
            .iter()
            .map(|p| self.get_llvm_type(&p.type_annotation))
            .collect::<Result<_>>()?;

        let fn_type = ret_type.fn_type(&param_types, false);
        let function = self.module.add_function(name, fn_type, None);

        // Create basic block
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        // Save current function
        let previous_function = self.current_function;
        self.current_function = Some(function);

        // Add parameters to scope
        self.named_values.clear();
        for (param, value) in params.iter().zip(function.get_param_iter()) {
            let alloca = self.create_entry_block_alloca(function, &param.name, value.get_type());
            self.builder.build_store(alloca, value);
            self.named_values.insert(param.name.clone(), alloca);
        }

        // Generate function body
        for node in body {
            self.visit_node(node)?;
        }

        // Verify function
        if function.verify(true) {
            // Restore previous function
            self.current_function = previous_function;
            Ok(function.as_basic_value_enum())
        } else {
            Err(IoError::runtime_error("Invalid generated function"))
        }
    }

    fn visit_node(&mut self, node: &ASTNode) -> Result<Self::Output> {
        match node {
            ASTNode::Function {
                name,
                params,
                return_type,
                body,
                is_async,
                location,
            } => self.visit_function(name, params, return_type, body, *is_async),
            ASTNode::BinaryOp { op, left, right } => {
                let lhs = self.visit_node(left)?;
                let rhs = self.visit_node(right)?;
                self.generate_binary_op(op, lhs, rhs)
            }
            ASTNode::IntegerLiteral(value) => Ok(self
                .context
                .i32_type()
                .const_int(*value as u64, false)
                .into()),
            ASTNode::Variable { name } => {
                if let Some(value) = self.named_values.get(name) {
                    Ok(self.builder.build_load(*value, name).into())
                } else {
                    Err(IoError::runtime_error("Unknown variable name"))
                }
            }
            ASTNode::VariableDeclaration {
                name,
                initializer,
                type_annotation,
            } => self.visit_variable_declaration(name, initializer, type_annotation),
            ASTNode::Return { value } => self.visit_return(value),
            ASTNode::If {
                condition,
                then_branch,
                else_branch,
            } => self.visit_if(condition, then_branch, else_branch),
            ASTNode::While { condition, body } => self.visit_while(condition, body),
            ASTNode::Call { name, args } => self.visit_call(name, args),
            _ => Err(IoError::runtime_error("Unimplemented node type")),
        }
    }

    fn visit_binary_op(
        &mut self,
        op: &BinaryOperator,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<Self::Output> {
        let lhs = self.visit_node(left)?;
        let rhs = self.visit_node(right)?;

        match op {
            BinaryOperator::Add => {
                let result = self.builder.build_int_add(
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "addtmp",
                );
                Ok(result.into())
            }
            BinaryOperator::Subtract => {
                let result = self.builder.build_int_sub(
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "subtmp",
                );
                Ok(result.into())
            }
            BinaryOperator::Multiply => {
                let result = self.builder.build_int_mul(
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "multmp",
                );
                Ok(result.into())
            }
            BinaryOperator::Divide => {
                let result = self.builder.build_int_signed_div(
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "divtmp",
                );
                Ok(result.into())
            }
            BinaryOperator::Equal => {
                let cmp = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "cmptmp",
                );
                let result =
                    self.builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "booltmp");
                Ok(result.into())
            }
            BinaryOperator::NotEqual => {
                let cmp = self.builder.build_int_compare(
                    inkwell::IntPredicate::NE,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "cmptmp",
                );
                let result =
                    self.builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "booltmp");
                Ok(result.into())
            }
            BinaryOperator::LessThan => {
                let cmp = self.builder.build_int_compare(
                    inkwell::IntPredicate::SLT,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "cmptmp",
                );
                let result =
                    self.builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "booltmp");
                Ok(result.into())
            }
            BinaryOperator::LessThanOrEqual => {
                let cmp = self.builder.build_int_compare(
                    inkwell::IntPredicate::SLE,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "cmptmp",
                );
                let result =
                    self.builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "booltmp");
                Ok(result.into())
            }
            BinaryOperator::GreaterThan => {
                let cmp = self.builder.build_int_compare(
                    inkwell::IntPredicate::SGT,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "cmptmp",
                );
                let result =
                    self.builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "booltmp");
                Ok(result.into())
            }
            BinaryOperator::GreaterThanOrEqual => {
                let cmp = self.builder.build_int_compare(
                    inkwell::IntPredicate::SGE,
                    lhs.into_int_value(),
                    rhs.into_int_value(),
                    "cmptmp",
                );
                let result =
                    self.builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "booltmp");
                Ok(result.into())
            }
            _ => Err(IoError::runtime_error("Unsupported binary operator")),
        }
    }

    fn visit_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &[ASTNode],
        else_branch: &Option<Vec<ASTNode>>,
    ) -> Result<Self::Output> {
        let cond_val = self.visit_node(condition)?;
        let current_fn = self
            .current_function
            .ok_or_else(|| IoError::runtime_error("If statement outside function"))?;

        let then_bb = self.context.append_basic_block(current_fn, "then");
        let else_bb = self.context.append_basic_block(current_fn, "else");
        let merge_bb = self.context.append_basic_block(current_fn, "merge");

        self.builder
            .build_conditional_branch(cond_val.into_int_value(), then_bb, else_bb);

        // Generate then block
        self.builder.position_at_end(then_bb);
        for node in then_branch {
            self.visit_node(node)?;
        }
        self.builder.build_unconditional_branch(merge_bb);

        // Generate else block
        self.builder.position_at_end(else_bb);
        if let Some(else_nodes) = else_branch {
            for node in else_nodes {
                self.visit_node(node)?;
            }
        }
        self.builder.build_unconditional_branch(merge_bb);

        // Continue in merge block
        self.builder.position_at_end(merge_bb);
        Ok(self.context.i32_type().const_zero().into())
    }

    fn visit_while(&mut self, condition: &ASTNode, body: &[ASTNode]) -> Result<Self::Output> {
        let current_fn = self
            .current_function
            .ok_or_else(|| IoError::runtime_error("While loop outside function"))?;

        let cond_bb = self.context.append_basic_block(current_fn, "while.cond");
        let body_bb = self.context.append_basic_block(current_fn, "while.body");
        let end_bb = self.context.append_basic_block(current_fn, "while.end");

        // Jump to condition block
        self.builder.build_unconditional_branch(cond_bb);

        // Generate condition
        self.builder.position_at_end(cond_bb);
        let cond_val = self.visit_node(condition)?;
        self.builder
            .build_conditional_branch(cond_val.into_int_value(), body_bb, end_bb);

        // Generate body
        self.builder.position_at_end(body_bb);
        for node in body {
            self.visit_node(node)?;
        }
        self.builder.build_unconditional_branch(cond_bb);

        // Continue after loop
        self.builder.position_at_end(end_bb);
        Ok(self.context.i32_type().const_zero().into())
    }

    fn visit_variable_declaration(
        &mut self,
        name: &str,
        initializer: &Option<ASTNode>,
        type_annotation: &Option<String>,
    ) -> Result<Self::Output> {
        let var_type = if let Some(type_name) = type_annotation {
            self.get_llvm_type(type_name)?
        } else if let Some(init) = initializer {
            // Infer type from initializer
            self.visit_node(init)?.get_type()
        } else {
            return Err(IoError::type_error("Cannot infer variable type"));
        };

        let alloca = self.create_entry_block_alloca(self.current_function.unwrap(), name, var_type);

        if let Some(init) = initializer {
            let init_val = self.visit_node(init)?;
            self.builder
                .build_store(alloca.into_pointer_value(), init_val);
        }

        self.named_values.insert(name.to_string(), alloca);
        Ok(alloca)
    }

    fn visit_return(&mut self, value: &Option<ASTNode>) -> Result<Self::Output> {
        let return_value = if let Some(expr) = value {
            Some(self.visit_node(expr)?)
        } else {
            None
        };

        match return_value {
            Some(val) => {
                self.builder.build_return(Some(&val));
                Ok(val)
            }
            None => {
                self.builder.build_return(None);
                Ok(self.context.i32_type().const_zero().into())
            }
        }
    }
}
