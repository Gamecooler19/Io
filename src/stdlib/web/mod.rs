pub mod http;
pub mod websocket;
pub mod server;

use crate::{error::IoError, Result};
use inkwell::values::FunctionValue;

pub struct WebModule<'ctx> {
    context: &'ctx inkwell::context::Context,
    http_client_type: inkwell::types::StructType<'ctx>,
    http_response_type: inkwell::types::StructType<'ctx>,
    websocket_type: inkwell::types::StructType<'ctx>,
    server_type: inkwell::types::StructType<'ctx>,
}

impl<'ctx> WebModule<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Result<Self> {
        let http_client_type = context.opaque_struct_type("HttpClient");
        let http_response_type = context.opaque_struct_type("HttpResponse");
        let websocket_type = context.opaque_struct_type("WebSocket");
        let server_type = context.opaque_struct_type("WebServer");

        // Initialize type layouts
        http_client_type.set_body(&[
            context.i8_ptr_type().into(), // URL
            context.i8_ptr_type().into(), // Headers
        ], false);

        http_response_type.set_body(&[
            context.i32_type().into(),    // Status code
            context.i8_ptr_type().into(), // Body
            context.i8_ptr_type().into(), // Headers
        ], false);

        Ok(Self {
            context,
            http_client_type,
            http_response_type,
            websocket_type,
            server_type,
        })
    }

    pub fn register_types(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        // Register web types in codegen
        codegen.add_type("HttpClient", self.http_client_type);
        codegen.add_type("HttpResponse", self.http_response_type);
        codegen.add_type("WebSocket", self.websocket_type);
        codegen.add_type("WebServer", self.server_type);
        Ok(())
    }
}
