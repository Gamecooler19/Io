use crate::{codegen::llvm::LLVMCodeGen, Result};
use inkwell::types::BasicType;
use inkwell::values::FunctionValue;
use inkwell::AddressSpace;

pub struct ConcurrentModule<'ctx> {
    context: &'ctx inkwell::context::Context,
    functions: std::collections::HashMap<String, FunctionValue<'ctx>>,
    mutex_type: Option<inkwell::types::StructType<'ctx>>,
    channel_type: Option<inkwell::types::StructType<'ctx>>,
}

impl<'ctx> ConcurrentModule<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Self {
        Self {
            context,
            functions: std::collections::HashMap::new(),
            mutex_type: None,
            channel_type: None,
        }
    }

    pub fn initialize(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        self.register_thread_functions(codegen)?;
        self.register_mutex_functions(codegen)?;
        self.register_channel_functions(codegen)?;
        self.register_mutex_type(codegen)?;
        self.register_channel_type(codegen)?;
        self.register_concurrent_operations(codegen)?;
        Ok(())
    }

    fn register_thread_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let void_type = self.context.void_type();
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::Generic);

        // Thread creation
        let spawn_fn = codegen.module.add_function(
            "thread_spawn",
            i8_ptr.fn_type(&[i8_ptr.into()], false),
            None,
        );
        self.functions.insert("thread_spawn".to_string(), spawn_fn);

        Ok(())
    }

    fn register_mutex_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let void_type = self.context.void_type();
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::Generic);

        // Mutex operations
        let mutex_new = codegen
            .module
            .add_function("mutex_new", i8_ptr.fn_type(&[], false), None);
        self.functions.insert("mutex_new".to_string(), mutex_new);

        Ok(())
    }

    fn register_channel_functions(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let void_type = self.context.void_type();
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::Generic);

        // Channel operations
        let channel_new =
            codegen
                .module
                .add_function("channel_new", i8_ptr.fn_type(&[], false), None);
        self.functions
            .insert("channel_new".to_string(), channel_new);

        Ok(())
    }

    fn register_mutex_type(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i8_ptr = codegen.context.ptr_type(AddressSpace::default());

        let mutex_type = codegen.context.struct_type(
            &[
                i8_ptr.into(),                      // data
                codegen.context.bool_type().into(), // locked
            ],
            false,
        );

        self.mutex_type = Some(mutex_type);
        Ok(())
    }

    fn register_channel_type(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        let i8_ptr = codegen.context.ptr_type(AddressSpace::default());
        let size_type = codegen.context.i64_type();

        let channel_type = codegen.context.struct_type(
            &[
                i8_ptr.into(),    // buffer
                size_type.into(), // capacity
                size_type.into(), // size
            ],
            false,
        );

        self.channel_type = Some(channel_type);
        Ok(())
    }

    fn register_concurrent_operations(&mut self, codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        self.register_mutex_operations(codegen)?;
        self.register_channel_operations(codegen)?;
        Ok(())
    }

    pub fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.functions.get(name).copied()
    }
}
