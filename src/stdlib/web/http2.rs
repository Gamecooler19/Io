use inkwell::values::FunctionValue;
use crate::{Result, error::IoError};

pub struct Http2Module<'ctx> {
    stream_type: inkwell::types::StructType<'ctx>,
    client_type: inkwell::types::StructType<'ctx>,
    create_client_fn: FunctionValue<'ctx>,
    create_stream_fn: FunctionValue<'ctx>,
    send_headers_fn: FunctionValue<'ctx>,
    send_data_fn: FunctionValue<'ctx>,
}

impl<'ctx> Http2Module<'ctx> {
    pub fn new(codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<Self> {
        let context = codegen.context;
        
        // Create HTTP/2 types
        let stream_type = context.opaque_struct_type("Http2Stream");
        let client_type = context.opaque_struct_type("Http2Client");

        stream_type.set_body(&[
            context.i32_type().into(),     // Stream ID
            context.i8_ptr_type().into(),  // Headers
            context.i8_ptr_type().into(),  // Data
            context.i32_type().into(),     // State
        ], false);

        client_type.set_body(&[
            context.i8_ptr_type().into(),  // Connection
            context.i32_type().into(),     // Max concurrent streams
            context.i8_ptr_type().into(),  // Settings
        ], false);

        // Create HTTP/2 functions
        let create_client_fn = Self::create_client_function(codegen, client_type)?;
        let create_stream_fn = Self::create_stream_function(codegen, stream_type)?;
        let send_headers_fn = Self::create_send_headers_function(codegen, stream_type)?;
        let send_data_fn = Self::create_send_data_function(codegen, stream_type)?;

        Ok(Self {
            stream_type,
            client_type,
            create_client_fn,
            create_stream_fn,
            send_headers_fn,
            send_data_fn,
        })
    }

    fn create_client_function(
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        client_type: inkwell::types::StructType<'ctx>,
    ) -> Result<FunctionValue<'ctx>> {
        let fn_type = client_type.fn_type(&[
            codegen.string_type().into(),  // Host
            codegen.i32_type().into(),     // Port
            codegen.i8_ptr_type().into(),  // Settings
        ], false);

        Ok(codegen.module.add_function("http2_create_client", fn_type, None))
    }

    pub fn generate_bindings(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        self.generate_client_binding(codegen)?;
        self.generate_stream_binding(codegen)?;
        Ok(())
    }
}
