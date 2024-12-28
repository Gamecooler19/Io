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
        // Implementation for SQL connection binding
        let builder = codegen.context.create_builder();
        let function = codegen.module.add_function(
            "connect_database",
            self.connect_fn.get_type(),
            None,
        );

        let entry = codegen.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        // RODO: Add connection validation and error handling
        let result = builder.build_call(
            self.connect_fn,
            &[
                function.get_nth_param(0).unwrap().into(),
                function.get_nth_param(1).unwrap().into(),
            ],
            "connection",
        );

        builder.build_return(Some(&result.try_as_basic_value().left().unwrap()));
        Ok(())
    }
}
