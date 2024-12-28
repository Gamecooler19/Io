pub mod schema;
pub mod resolver;
pub mod types;

use inkwell::values::FunctionValue;
use crate::{Result, error::IoError};

pub struct GraphQLModule<'ctx> {
    context: &'ctx inkwell::context::Context,
    schema_type: inkwell::types::StructType<'ctx>,
    resolver_type: inkwell::types::StructType<'ctx>,
    query_type: inkwell::types::StructType<'ctx>,
}

impl<'ctx> GraphQLModule<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Result<Self> {
        let schema_type = context.opaque_struct_type("GraphQLSchema");
        let resolver_type = context.opaque_struct_type("GraphQLResolver");
        let query_type = context.opaque_struct_type("GraphQLQuery");

        schema_type.set_body(&[
            context.i8_ptr_type().into(), // Schema definition
            context.i8_ptr_type().into(), // Resolvers
        ], false);

        Ok(Self {
            context,
            schema_type,
            resolver_type,
            query_type,
        })
    }

    pub fn register_types(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<()> {
        codegen.add_type("GraphQLSchema", self.schema_type);
        codegen.add_type("GraphQLResolver", self.resolver_type);
        codegen.add_type("GraphQLQuery", self.query_type);
        Ok(())
    }
}
