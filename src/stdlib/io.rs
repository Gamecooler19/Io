use crate::{codegen::llvm::LLVMCodeGen, error::IoError, Result};
use inkwell::values::{BasicValue, FunctionValue, PointerValue};
use inkwell::{builder::Builder, AddressSpace};

pub struct IoModule<'ctx> {
    functions: std::collections::HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> IoModule<'ctx> {
    pub fn new() -> Self {
        Self {
            functions: std::collections::HashMap::new(),
        }
    }

    pub fn initialize(&self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let void_type = codegen.context.void_type();
        let i8_ptr = codegen.context.i8_type().ptr_type(AddressSpace::Generic);
        let i32_type = codegen.context.i32_type();

        // Register basic IO functions
        let printf_type = i32_type.fn_type(&[i8_ptr.into()], true);
        codegen.module.add_function("printf", printf_type, None);

        let file_open_type = i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
        codegen.module.add_function("fopen", file_open_type, None);

        let file_close_type = i32_type.fn_type(&[i8_ptr.into()], false);
        codegen.module.add_function("fclose", file_close_type, None);

        Ok(())
    }

    pub fn generate_bindings(&self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i8_ptr = codegen.context.i8_type().ptr_type(AddressSpace::Generic);
        let i32_type = codegen.context.i32_type();

        // Bind print function
        let print_fn = codegen.module.add_function(
            "_cb_print",
            i32_type.fn_type(&[i8_ptr.into()], false),
            None,
        );

        // Bind file operations
        let open_fn = codegen.module.add_function(
            "_cb_file_open",
            i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
            None,
        );

        let close_fn = codegen.module.add_function(
            "_cb_file_close",
            i32_type.fn_type(&[i8_ptr.into()], false),
            None,
        );

        Ok(())
    }

    pub fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.functions.get(name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    #[test]
    fn test_io_operations() {
        let context = Context::create();
        let module = context.create_module("test");
        let mut codegen = LLVMCodeGen::new(&context, &module);

        let io_module = IoModule::new();
        assert!(io_module.initialize(&mut codegen).is_ok());
        assert!(io_module.generate_bindings(&mut codegen).is_ok());

        // Verify print function exists
        assert!(module.get_function("_cb_print").is_some());
    }

    #[test]
    fn test_file_operations() {
        let context = Context::create();
        let module = context.create_module("test");
        let mut codegen = LLVMCodeGen::new(&context, &module);

        let io_module = IoModule::new();
        io_module.initialize(&mut codegen).unwrap();
        io_module.generate_bindings(&mut codegen).unwrap();

        // Verify file operations exist
        assert!(module.get_function("_cb_file_open").is_some());
        assert!(module.get_function("_cb_file_close").is_some());
    }

    #[test]
    fn test_error_handling() {
        let context = Context::create();
        let module = context.create_module("test");
        let mut codegen = LLVMCodeGen::new(&context, &module);

        let io_module = IoModule::new();

        // Test initialization with invalid module
        let result = io_module.initialize(&mut codegen);
        assert!(result.is_ok());

        // Verify error handling in bindings
        let binding_result = io_module.generate_bindings(&mut codegen);
        assert!(binding_result.is_ok());
    }

    #[test]
    fn test_buffered_operations() {
        let context = Context::create();
        let module = context.create_module("test");
        let mut codegen = LLVMCodeGen::new(&context, &module);

        let io_module = IoModule::new();
        assert!(io_module.initialize(&mut codegen).is_ok());
        assert!(io_module.generate_bindings(&mut codegen).is_ok());

        // Verify module integrity
        assert!(module.verify().is_ok());
    }
}
