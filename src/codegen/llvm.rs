use crate::codegen::debug::{DebugInfo, SourceLocation};
use crate::{
    ast::{ASTNode, BinaryOperator, Function, Module as AstModule},
    error::IoError,
    Result,
};
use inkwell::{
    builder::Builder,
    context::Context,
    debug_info::{
        DIFile, DIFlags, DIScope, DISubprogram, DWARFEmissionKind, DWARFSourceLanguage,
        DebugInfoBuilder,
    },
    module::Module,
    passes::PassManager,
    targets::{CodeModel, FileType, RelocMode, Target, TargetMachine},
    types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum},
    values::{BasicValue, BasicValueEnum, CallSiteValue, FunctionValue},
    AddressSpace, OptimizationLevel,
};
use std::collections::HashMap; // Add if not imported

pub struct LLVMCodeGen<'ctx> {
    pub(crate) context: &'ctx Context,
    pub(crate) module: Module<'ctx>,
    pub(crate) builder: Builder<'ctx>,
    named_values: HashMap<String, BasicValueEnum<'ctx>>,
    current_function: Option<FunctionValue<'ctx>>,
    optimization_level: OptimizationLevel,
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
                    BinaryOperator::Modulo => {
                        self.builder.build_int_signed_rem(l, r, "modtmp").into()
                    }
                    BinaryOperator::BitwiseAnd => self.builder.build_and(l, r, "andtmp").into(),
                    BinaryOperator::BitwiseOr => self.builder.build_or(l, r, "ortmp").into(),
                    BinaryOperator::BitwiseXor => self.builder.build_xor(l, r, "xortmp").into(),
                    BinaryOperator::LeftShift => {
                        self.builder.build_left_shift(l, r, "lshifttmp").into()
                    }
                    BinaryOperator::RightShift => self
                        .builder
                        .build_right_shift(l, r, false, "rshifttmp")
                        .into(),
                    _ => return Err(IoError::runtime_error("Unsupported binary operator")),
                })
            }
            (l, r) if l.is_float_type() && r.is_float_type() => {
                let l = left.into_float_value();
                let r = right.into_float_value();
                Ok(match op {
                    BinaryOperator::Add => self.builder.build_float_add(l, r, "faddtmp").into(),
                    BinaryOperator::Subtract => {
                        self.builder.build_float_sub(l, r, "fsubtmp").into()
                    }
                    BinaryOperator::Multiply => {
                        self.builder.build_float_mul(l, r, "fmultmp").into()
                    }
                    BinaryOperator::Divide => self.builder.build_float_div(l, r, "fdivtmp").into(),
                    BinaryOperator::Modulo => self.builder.build_float_rem(l, r, "fmodtmp").into(),
                    _ => return Err(IoError::runtime_error("Unsupported float operation")),
                })
            }
            (l, r) if l.is_struct_type() && r.is_struct_type() => {
                self.handle_struct_operation(op, left, right)
            }
            (l, r) if l.is_array_type() && r.is_array_type() => {
                self.handle_array_operation(op, left, right)
            }
            (l, r) if l.is_vector_type() && r.is_vector_type() => {
                self.handle_vector_operation(op, left, right)
            }
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
            file,
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
            "memset" => self.module.add_function(
                "llvm.memset.p0i8.i64",
                self.context.void_type().fn_type(
                    &[
                        self.context
                            .i8_type()
                            .ptr_type(AddressSpace::Generic)
                            .into(),
                        self.context.i8_type().into(),
                        self.context.i64_type().into(),
                        self.context.bool_type().into(),
                    ],
                    false,
                ),
                None,
            ),
            "sqrt" => self.module.add_function(
                "llvm.sqrt.f64",
                self.f64_type().fn_type(&[self.f64_type().into()], false),
                None,
            ),
            "sin" => self.module.add_function(
                "llvm.sin.f64",
                self.f64_type().fn_type(&[self.f64_type().into()], false),
                None,
            ),
            "cos" => self.module.add_function(
                "llvm.cos.f64",
                self.f64_type().fn_type(&[self.f64_type().into()], false),
                None,
            ),
            "pow" => self.module.add_function(
                "llvm.pow.f64",
                self.f64_type()
                    .fn_type(&[self.f64_type().into(), self.f64_type().into()], false),
                None,
            ),
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
        // Create function type
        let fn_type = ret_type.fn_type(param_types, false);

        // Create the function
        let function =
            self.module
                .add_function(&format!("func_{}", self.function_counter), fn_type, None);
        self.function_counter += 1;

        // Create entry block
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        // Set up function parameters
        for (i, param) in function.get_param_iter().enumerate() {
            let param_name = format!("param_{}", i);
            let alloca = self.builder.build_alloca(param.get_type(), &param_name);
            self.builder.build_store(alloca, param);
            self.variables.insert(param_name, alloca.into());
        }

        // Add debug info if enabled
        if self.debug_info_enabled {
            self.add_function_debug_info(&function)?;
        }

        // Add function to optimization pass manager
        self.function_pass_manager.run_on(&function);

        Ok(())
    }

    fn add_function_debug_info(&self, function: &FunctionValue<'ctx>) -> Result<()> {
        let file = self
            .debug_builder
            .create_file(&self.current_file, &self.current_directory);

        let scope = self.debug_builder.create_function(
            file,
            &function.get_name().to_string_lossy(),
            None,
            file,
            self.current_line,
            false,
            true,
            self.current_line,
            0,
            false,
        );

        self.debug_builder.set_current_debug_location(
            self.current_line,
            self.current_column,
            scope,
            None,
        );

        Ok(())
    }

    pub fn build_struct_gep(
        &self,
        ptr: inkwell::values::PointerValue<'ctx>,
        field_idx: u32,
        name: &str,
    ) -> Result<inkwell::values::PointerValue<'ctx>> {
        unsafe {
            self.builder
                .build_struct_gep(ptr, field_idx, name)
                .map_err(IoError::from)
        }
    }

    pub fn build_load(
        &self,
        ptr: inkwell::values::PointerValue<'ctx>,
        name: &str,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>> {
        self.builder
            .build_load(ptr.get_type(), ptr, name)
            .map_err(IoError::from)
    }

    pub fn build_call(
        &self,
        function: FunctionValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        name: &str,
    ) -> Result<CallSiteValue<'ctx>> {
        self.builder
            .build_call(function, args, name)
            .map_err(IoError::from)
    }

    fn build_binary_op(
        &self,
        op: &BinaryOperator,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let result = match op {
            BinaryOperator::Add => {
                let result = self.builder.build_int_add(
                    left.into_int_value(),
                    right.into_int_value(),
                    "addtmp",
                );
                Ok(result.into())
            }
            BinaryOperator::Subtract => {
                let result = self.builder.build_int_sub(
                    left.into_int_value(),
                    right.into_int_value(),
                    "subtmp",
                );
                Ok(result.into())
            }
            BinaryOperator::Multiply => {
                let result = self.builder.build_int_mul(
                    left.into_int_value(),
                    right.into_int_value(),
                    "multmp",
                );
                Ok(result.into())
            }
            BinaryOperator::Divide => {
                let result = self.builder.build_int_signed_div(
                    left.into_int_value(),
                    right.into_int_value(),
                    "divtmp",
                );
                Ok(result.into())
            }
            BinaryOperator::Equal => {
                let cmp = self.builder.build_int_compare(
                    inkwell::IntPredicate::EQ,
                    left.into_int_value(),
                    right.into_int_value(),
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
                    left.into_int_value(),
                    right.into_int_value(),
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
                    left.into_int_value(),
                    right.into_int_value(),
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
                    left.into_int_value(),
                    right.into_int_value(),
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
                    left.into_int_value(),
                    right.into_int_value(),
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
                    left.into_int_value(),
                    right.into_int_value(),
                    "cmptmp",
                );
                let result =
                    self.builder
                        .build_int_z_extend(cmp, self.context.i32_type(), "booltmp");
                Ok(result.into())
            }
            BinaryOperator::BitAnd => self.builder.build_and(
                left.into_int_value(),
                right.into_int_value(),
                "bitandtmp",
            )?,
            BinaryOperator::BitOr => {
                self.builder
                    .build_or(left.into_int_value(), right.into_int_value(), "bitortmp")?
            }
            BinaryOperator::BitXor => self.builder.build_xor(
                left.into_int_value(),
                right.into_int_value(),
                "bitxortmp",
            )?,
            BinaryOperator::LeftShift => self.builder.build_left_shift(
                left.into_int_value(),
                right.into_int_value(),
                "shltmp",
            )?,
            BinaryOperator::RightShift => self.builder.build_right_shift(
                left.into_int_value(),
                right.into_int_value(),
                false,
                "shrtmp",
            )?,
            BinaryOperator::Power => {
                let pow_intrinsic = self.get_or_insert_intrinsic("llvm.pow")?;
                let call = self.builder.build_call(
                    pow_intrinsic,
                    &[
                        left.into_float_value().into(),
                        right.into_float_value().into(),
                    ],
                    "powtmp",
                )?;
                call.try_as_basic_value().left().unwrap()
            }
            _ => Err(IoError::runtime_error("Unsupported binary operator")),
        };

        Ok(result.into())
    }

    fn build_function_call(
        &self,
        function: FunctionValue<'ctx>,
        args: &[BasicValueEnum<'ctx>],
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        let metadata_args: Vec<BasicMetadataTypeEnum> =
            args.iter().map(|&arg| arg.into()).collect();

        let call_site = self
            .builder
            .build_call(function, &metadata_args, name)
            .map_err(IoError::from)?;

        self.convert_to_basic_value(call_site)
    }

    fn convert_to_basic_value(
        &self,
        value: inkwell::values::CallSiteValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        value
            .try_as_basic_value()
            .left()
            .ok_or_else(|| IoError::codegen_error("Failed to convert call result to basic value"))
    }

    pub fn array_type(&self, element_type: BasicTypeEnum<'ctx>, size: u32) -> BasicTypeEnum<'ctx> {
        element_type.array_type(size).as_basic_type_enum()
    }

    pub fn struct_type(
        &self,
        field_types: &[BasicTypeEnum<'ctx>],
        name: Option<&str>,
    ) -> BasicTypeEnum<'ctx> {
        let struct_type = self.context.struct_type(field_types, false);
        if let Some(name) = name {
            struct_type.set_name(name);
        }
        struct_type.as_basic_type_enum()
    }

    pub fn vector_type(&self, element_type: BasicTypeEnum<'ctx>, size: u32) -> BasicTypeEnum<'ctx> {
        element_type.vec_type(size).as_basic_type_enum()
    }

    fn convert_type(
        &self,
        value: BasicValueEnum<'ctx>,
        target_type: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        match (value.get_type(), target_type) {
            (t1, t2) if t1.is_int_type() && t2.is_int_type() => {
                let from_bits = t1.into_int_type().get_bit_width();
                let to_bits = t2.into_int_type().get_bit_width();
                Ok(if from_bits < to_bits {
                    self.builder
                        .build_int_s_extend(value.into_int_value(), t2.into_int_type(), "ext")
                        .into()
                } else {
                    self.builder
                        .build_int_truncate(value.into_int_value(), t2.into_int_type(), "trunc")
                        .into()
                })
            }
            (t1, t2) if t1.is_float_type() && t2.is_float_type() => Ok(self
                .builder
                .build_float_cast(value.into_float_value(), t2.into_float_type(), "fcast")
                .into()),
            (t1, t2) if t1.is_int_type() && t2.is_float_type() => Ok(self
                .builder
                .build_signed_int_to_float(
                    value.into_int_value(),
                    t2.into_float_type(),
                    "int2float",
                )
                .into()),
            (t1, t2) if t1.is_float_type() && t2.is_int_type() => Ok(self
                .builder
                .build_float_to_signed_int(
                    value.into_float_value(),
                    t2.into_int_type(),
                    "float2int",
                )
                .into()),
            _ => Err(IoError::type_error("Unsupported type conversion")),
        }
    }

    fn handle_struct_operation(
        &self,
        op: &BinaryOperator,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        match op {
            BinaryOperator::Equal => self.build_struct_comparison(left, right, true),
            BinaryOperator::NotEqual => self.build_struct_comparison(left, right, false),
            _ => Err(IoError::runtime_error("Unsupported struct operation")),
        }
    }

    fn handle_array_operation(
        &self,
        op: &BinaryOperator,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        match op {
            BinaryOperator::Add => self.build_array_concatenation(left, right),
            _ => Err(IoError::runtime_error("Unsupported array operation")),
        }
    }

    fn handle_vector_operation(
        &self,
        op: &BinaryOperator,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let l = left.into_vector_value();
        let r = right.into_vector_value();
        Ok(match op {
            BinaryOperator::Add => self.builder.build_vector_add(l, r, "vector_add").into(),
            BinaryOperator::Multiply => self.builder.build_vector_mul(l, r, "vector_mul").into(),
            _ => return Err(IoError::runtime_error("Unsupported vector operation")),
        })
    }

    fn convert_value(
        &self,
        value: BasicValueEnum<'ctx>,
        target_type: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        match (value.get_type(), target_type) {
            // Integer conversions
            (t1, t2) if t1.is_int_type() && t2.is_int_type() => {
                let source_bits = t1.into_int_type().get_bit_width();
                let target_bits = t2.into_int_type().get_bit_width();

                Ok(if source_bits < target_bits {
                    self.builder
                        .build_int_s_extend(value.into_int_value(), t2.into_int_type(), "ext")
                        .into()
                } else if source_bits > target_bits {
                    self.builder
                        .build_int_truncate(value.into_int_value(), t2.into_int_type(), "trunc")
                        .into()
                } else {
                    value
                })
            }
            // Float conversions
            (t1, t2) if t1.is_float_type() && t2.is_float_type() => Ok(self
                .builder
                .build_float_cast(value.into_float_value(), t2.into_float_type(), "float_cast")
                .into()),
            // Integer to float
            (t1, t2) if t1.is_int_type() && t2.is_float_type() => Ok(self
                .builder
                .build_signed_int_to_float(
                    value.into_int_value(),
                    t2.into_float_type(),
                    "int2float",
                )
                .into()),
            // Float to integer
            (t1, t2) if t1.is_float_type() && t2.is_int_type() => Ok(self
                .builder
                .build_float_to_signed_int(
                    value.into_float_value(),
                    t2.into_int_type(),
                    "float2int",
                )
                .into()),
            // Pointer conversions
            (t1, t2) if t1.is_pointer_type() && t2.is_pointer_type() => Ok(self
                .builder
                .build_pointer_cast(
                    value.into_pointer_value(),
                    t2.into_pointer_type(),
                    "ptr_cast",
                )
                .into()),
            _ => Err(IoError::type_error(format!(
                "Unsupported type conversion from {:?} to {:?}",
                value.get_type(),
                target_type
            ))),
        }
    }
}

impl<'ctx> LLVMCodeGen<'ctx> {
    fn visit_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_type: &Option<String>,
        body: &[ASTNode],
        is_async: bool,
    ) -> Result<BasicValueEnum<'ctx>> {
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

    fn visit_node(&mut self, node: &ASTNode) -> Result<BasicValueEnum<'ctx>> {
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
    ) -> Result<BasicValueEnum<'ctx>> {
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
            BinaryOperator::BitAnd => {
                self.builder
                    .build_and(lhs.into_int_value(), rhs.into_int_value(), "bitandtmp")?
            }
            BinaryOperator::BitOr => {
                self.builder
                    .build_or(lhs.into_int_value(), rhs.into_int_value(), "bitortmp")?
            }
            BinaryOperator::BitXor => {
                self.builder
                    .build_xor(lhs.into_int_value(), rhs.into_int_value(), "bitxortmp")?
            }
            BinaryOperator::LeftShift => self.builder.build_left_shift(
                lhs.into_int_value(),
                rhs.into_int_value(),
                "shltmp",
            )?,
            BinaryOperator::RightShift => self.builder.build_right_shift(
                lhs.into_int_value(),
                rhs.into_int_value(),
                false,
                "shrtmp",
            )?,
            BinaryOperator::Power => {
                let pow_intrinsic = self.get_or_insert_intrinsic("llvm.pow")?;
                let call = self.builder.build_call(
                    pow_intrinsic,
                    &[lhs.into_float_value().into(), rhs.into_float_value().into()],
                    "powtmp",
                )?;
                call.try_as_basic_value().left().unwrap()
            }
            _ => Err(IoError::runtime_error("Unsupported binary operator")),
        }
    }

    fn visit_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &[ASTNode],
        else_branch: &Option<Vec<ASTNode>>,
    ) -> Result<BasicValueEnum<'ctx>> {
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

    fn visit_while(
        &mut self,
        condition: &ASTNode,
        body: &[ASTNode],
    ) -> Result<BasicValueEnum<'ctx>> {
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
    ) -> Result<BasicValueEnum<'ctx>> {
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

    fn visit_return(&mut self, value: &Option<ASTNode>) -> Result<BasicValueEnum<'ctx>> {
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

pub struct LLVMImplementation<'ctx> {
    pub context: &'ctx Context,
}

impl<'ctx> LLVMImplementation<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self { context }
    }

    pub fn finalize(&self) -> Result<(), IoError> {
        // Verify module integrity
        self.verify_module()?;

        // Run optimization passes
        self.run_optimization_passes()?;

        // Generate final output
        self.generate_output()?;

        Ok(())
    }

    fn verify_module(&self) -> Result<(), String> {
        // Verify each function
        for function in self.module.get_functions() {
            if !function.verify(true) {
                return Err(format!(
                    "Function verification failed: {}",
                    function.get_name().to_string_lossy()
                ));
            }
        }

        // Verify the entire module
        if self.module.verify().is_err() {
            return Err("Module verification failed".to_string());
        }

        // Check for unresolved symbols
        if let Some(missing) = self.find_unresolved_symbols() {
            return Err(format!("Unresolved symbols found: {:?}", missing));
        }

        Ok(())
    }

    fn run_optimization_passes(&self) -> Result<(), IoError> {
        // Create module pass manager
        let pass_manager = PassManager::create(());

        // Add analysis passes
        pass_manager.add_promote_memory_to_register_pass();
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_reassociate_pass();
        pass_manager.add_gvn_pass();
        pass_manager.add_cfg_simplification_pass();

        // Add aggressive optimization passes if enabled
        if self.optimization_level >= OptimizationLevel::Aggressive {
            pass_manager.add_function_inlining_pass();
            pass_manager.add_global_dce_pass();
            pass_manager.add_constant_propagation_pass();
            pass_manager.add_dead_store_elimination_pass();
            pass_manager.add_aggressive_dce_pass();

            // Loop optimizations
            pass_manager.add_loop_unroll_pass();
            pass_manager.add_loop_vectorize_pass();
            pass_manager.add_slp_vectorize_pass();
            pass_manager.add_loop_deletion_pass();

            // More aggressive optimizations
            pass_manager.add_tail_call_elimination_pass();
            pass_manager.add_memcpy_optimize_pass();
            pass_manager.add_bit_tracking_dce_pass();
            pass_manager.add_partial_inlining_pass();
        }

        // Run the pass manager
        pass_manager.run_on(&self.module);

        Ok(())
    }

    fn generate_output(&self) -> Result<(), IoError> {
        // Generate object file
        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .map_err(|e| IoError::codegen_error(format!("Failed to get target: {}", e)))?;

        let target_machine = target
            .create_target_machine(
                &target_triple,
                "generic",
                "",
                OptimizationLevel::Default,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or_else(|| IoError::codegen_error("Failed to create target machine"))?;

        // Write object file
        target_machine
            .write_to_file(&self.module, FileType::Object, "output.o")
            .map_err(|e| IoError::codegen_error(format!("Failed to write object file: {}", e)))?;

        // Generate LLVM IR for debugging
        if self.emit_ir {
            self.module
                .print_to_file("output.ll")
                .map_err(|e| IoError::codegen_error(format!("Failed to write IR file: {}", e)))?;
        }

        Ok(())
    }

    fn find_unresolved_symbols(&self) -> Option<Vec<String>> {
        let mut unresolved = Vec::new();

        for function in self.module.get_functions() {
            if function.is_declaration() {
                unresolved.push(function.get_name().to_string_lossy().to_string());
            }
        }

        if unresolved.is_empty() {
            None
        } else {
            Some(unresolved)
        }
    }
}
