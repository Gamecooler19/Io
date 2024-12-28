use inkwell::values::{FunctionValue, BasicValueEnum};
use crate::{Result, error::IoError};

pub struct WebServer<'ctx> {
    create_server_fn: FunctionValue<'ctx>,
    add_route_fn: FunctionValue<'ctx>,
    start_server_fn: FunctionValue<'ctx>,
}

impl<'ctx> WebServer<'ctx> {
    pub fn new(codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<Self> {
        let server_type = codegen.get_type("WebServer")?;
        
        // Create server functions
        let create_server_fn = Self::create_server_function(codegen, server_type)?;
        let add_route_fn = Self::create_add_route_function(codegen, server_type)?;
        let start_server_fn = Self::create_start_server_function(codegen, server_type)?;

        Ok(Self {
            create_server_fn,
            add_route_fn,
            start_server_fn,
        })
    }

    fn create_server_function(
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        server_type: BasicValueEnum<'ctx>,
    ) -> Result<FunctionValue<'ctx>> {
        let fn_type = server_type.into_struct_type().fn_type(&[
            codegen.string_type().into(), // Host
            codegen.i32_type().into(),    // Port
        ], false);

        Ok(codegen.module.add_function("create_server", fn_type, None))
    }

    fn create_add_route_function(
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        server_type: BasicValueEnum<'ctx>,
    ) -> Result<FunctionValue<'ctx>> {
        let fn_type = codegen.void_type().fn_type(&[
            server_type.into_struct_type().into(),  // Server instance
            codegen.string_type().into(),          // Path
            codegen.string_type().into(),          // Method
            codegen.function_type().into(),        // Handler function
        ], false);

        Ok(codegen.module.add_function("add_route", fn_type, None))
    }

    pub fn generate_bindings(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        // Generate high-level Io bindings for server functions
        self.generate_server_creation_binding(codegen)?;
        self.generate_route_binding(codegen)?;
        Ok(())
    }

    fn generate_server_creation_binding(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        let builder = codegen.context.create_builder();
        let function = codegen.module.add_function(
            "create_web_server",
            self.create_server_fn.get_type(),
            None,
        );

        let entry = codegen.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        // Add error handling and server initialization
        let result = builder.build_call(
            self.create_server_fn,
            &[
                function.get_nth_param(0).unwrap().into(),
                function.get_nth_param(1).unwrap().into(),
            ],
            "server_instance",
        );

        builder.build_return(Some(&result.try_as_basic_value().left().unwrap()));
        Ok(())
    }
}
