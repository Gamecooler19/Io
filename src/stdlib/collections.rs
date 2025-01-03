use crate::{codegen::llvm::LLVMCodeGen, Result};
use inkwell::{context::Context, types::BasicType, values::FunctionValue, AddressSpace};
use std::collections::HashMap;

pub struct CollectionsModule<'ctx> {
    context: &'ctx Context,
    functions: HashMap<String, FunctionValue<'ctx>>,
    vector_type: Option<inkwell::types::StructType<'ctx>>,
    map_type: Option<inkwell::types::StructType<'ctx>>,
}

impl<'ctx> CollectionsModule<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            functions: HashMap::new(),
            vector_type: None,
            map_type: None,
        }
    }

    pub fn get_function(&self, _name: &str) -> Option<FunctionValue<'ctx>> {
        None
    }

    pub fn generate_bindings(
        &self,
        _codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn initialize(&self, _codegen: &mut LLVMCodeGen<'ctx>) -> Result<()> {
        Ok(())
    }
}
