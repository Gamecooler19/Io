use inkwell::values::FunctionValue;
use crate::{Result, error::IoError};

pub struct SqlModule<'ctx> {
    connect_fn: FunctionValue<'ctx>,
    execute_fn: FunctionValue<'ctx>,
    prepare_fn: FunctionValue<'ctx>,
    transaction_fn: FunctionValue<'ctx>,
}

impl<'ctx> SqlModule<'ctx> {
    pub fn new(codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<Self> {
        let connect_fn = Self::create_connect_function(codegen)?;
        let execute_fn = Self::create_execute_function(codegen)?;
        let prepare_fn = Self::create_prepare_function(codegen)?;
        let transaction_fn = Self::create_transaction_function(codegen)?;

        Ok(Self {
            connect_fn,
            execute_fn,
            prepare_fn,
            transaction_fn,
        })
    }

    fn create_connect_function(
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<FunctionValue<'ctx>> {
        let connection_type = codegen.get_type("DbConnection")?;
        let fn_type = connection_type.fn_type(&[
            codegen.string_type().into(), // Connection string
            codegen.string_type().into(), // Driver type
        ], false);

        Ok(codegen.module.add_function("sql_connect", fn_type, None))
    }

    pub fn generate_bindings(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        self.generate_connect_binding(codegen)?;
        self.generate_query_binding(codegen)?;
        self.generate_transaction_binding(codegen)?;
        Ok(())
    }

    fn generate_connect_binding(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        let builder = codegen.context.create_builder();
        let i8_ptr_ty = codegen.context.i8_type().ptr_type(inkwell::AddressSpace::Generic);
        let bool_ty = codegen.context.bool_type();
        let connection_ty = codegen.get_type("DbConnection")?;

        // Create function type for connection validation
        let validate_fn_type = bool_ty.fn_type(&[i8_ptr_ty.into(), i8_ptr_ty.into()], false);
        let validate_fn = codegen.module.add_function(
            "validate_connection_params",
            validate_fn_type,
            None,
        );

        // Create function for database connection
        let connect_fn = codegen.module.add_function(
            "connect_database",
            connection_ty.fn_type(&[i8_ptr_ty.into(), i8_ptr_ty.into()], false),
            None,
        );

        let entry = codegen.context.append_basic_block(connect_fn, "entry");
        let validation_failed = codegen.context.append_basic_block(connect_fn, "validation_failed");
        let connect_db = codegen.context.append_basic_block(connect_fn, "connect_db");
        let error_handle = codegen.context.append_basic_block(connect_fn, "error_handle");
        let return_block = codegen.context.append_basic_block(connect_fn, "return");

        builder.position_at_end(entry);

        // Get function parameters
        let conn_string = connect_fn.get_nth_param(0).unwrap();
        let driver_type = connect_fn.get_nth_param(1).unwrap();

        // Validate connection parameters
        let validation_result = builder.build_call(
            validate_fn,
            &[conn_string.into(), driver_type.into()],
            "validate",
        );

        // Branch based on validation result
        builder.build_conditional_branch(
            validation_result.try_as_basic_value().left().unwrap().into_int_value(),
            connect_db,
            validation_failed,
        );

        // Handle validation failure
        builder.position_at_end(validation_failed);
        self.build_error_return(
            &builder,
            codegen,
            "Invalid connection parameters",
            "VALIDATION_ERROR",
        );
        builder.build_unconditional_branch(error_handle);

        // Attempt database connection
        builder.position_at_end(connect_db);
        let connection_result = builder.build_call(
            self.connect_fn,
            &[conn_string.into(), driver_type.into()],
            "connection",
        );

        // Check connection success
        let conn_success = builder.build_is_not_null(
            connection_result.try_as_basic_value().left().unwrap().into_pointer_value(),
            "conn_check",
        );

        builder.build_conditional_branch(conn_success, return_block, error_handle);

        // Handle connection error
        builder.position_at_end(error_handle);
        self.build_connection_error_handling(&builder, codegen);
        builder.build_unconditional_branch(return_block);

        // Return connection or error
        builder.position_at_end(return_block);
        let phi = builder.build_phi(
            connection_ty,
            "result",
        );

        phi.add_incoming(&[
            (&connection_result.try_as_basic_value().left().unwrap(), connect_db),
            (&codegen.context.i8_type().ptr_type(inkwell::AddressSpace::Generic).const_null(), error_handle),
        ]);

        builder.build_return(Some(&phi.as_basic_value()));

        Ok(())
    }

    fn build_error_return(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        codegen: &crate::codegen::llvm::LLVMCodeGen<'ctx>,
        message: &str,
        error_code: &str,
    ) {
        let error_msg = builder.build_global_string_ptr(message, "error_msg");
        let error_code = builder.build_global_string_ptr(error_code, "error_code");
        
        let error_handler = codegen.module.get_function("handle_sql_error")
            .expect("SQL error handler not found");

        builder.build_call(
            error_handler,
            &[
                error_msg.as_pointer_value().into(),
                error_code.as_pointer_value().into(),
            ],
            "error",
        );
    }

    fn build_connection_error_handling(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        codegen: &crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) {
        let get_error = codegen.module.get_function("get_last_sql_error")
            .expect("SQL error getter not found");
        
        let error_info = builder.build_call(get_error, &[], "error_info");
        
        // Log error details
        let log_error = codegen.module.get_function("log_sql_error")
            .expect("SQL error logger not found");
        
        builder.build_call(
            log_error,
            &[error_info.try_as_basic_value().left().unwrap()],
            "log",
        );

        // Set error context
        let error_context = builder.build_call(
            codegen.module.get_function("create_sql_error_context")
                .expect("Error context creator not found"),
            &[error_info.try_as_basic_value().left().unwrap()],
            "context",
        );

        // Store error context
        let store_context = codegen.module.get_function("store_error_context")
            .expect("Context store function not found");
        
        builder.build_call(
            store_context,
            &[error_context.try_as_basic_value().left().unwrap()],
            "store",
        );
    }

    fn create_sql_error_helpers(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        // Create error info type
        let error_info_type = codegen.context.struct_type(
            &[
                codegen.i32_type().into(),  // error code
                codegen.string_type().into(), // message
                codegen.string_type().into(), // sql state
            ],
            false,
        );
        codegen.register_type("SqlErrorInfo", error_info_type.into())?;

        // Create error context type
        let context_type = codegen.context.struct_type(
            &[
                error_info_type.into(),
                codegen.string_type().into(), // query
                codegen.i64_type().into(),    // timestamp
            ],
            false,
        );
        codegen.register_type("SqlErrorContext", context_type.into())?;

        Ok(())
    }
}
