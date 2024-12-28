use inkwell::values::FunctionValue;
use crate::{Result, error::IoError};

pub struct ResolverBuilder<'ctx> {
    context: &'ctx inkwell::context::Context,
    resolvers: Vec<Resolver<'ctx>>,
}

impl<'ctx> ResolverBuilder<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Self {
        Self {
            context,
            resolvers: Vec::new(),
        }
    }

    pub fn add_resolver(
        &mut self,
        type_name: &str,
        field_name: &str,
        resolver_fn: FunctionValue<'ctx>,
    ) -> Result<()> {
        self.resolvers.push(Resolver {
            type_name: type_name.to_string(),
            field_name: field_name.to_string(),
            resolver_fn,
        });
        Ok(())
    }

    pub fn generate_resolver_map(&self, codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>) -> Result<FunctionValue<'ctx>> {
        let map_type = codegen.get_type("GraphQLResolverMap")?;
        let fn_type = map_type.fn_type(&[], false);
        
        let function = codegen.module.add_function("create_resolver_map", fn_type, None);
        let builder = codegen.context.create_builder();
        let entry = codegen.context.append_basic_block(function, "entry");
        
        builder.position_at_end(entry);

        // Generate resolver map initialization
        let map = self.generate_resolver_map_initialization(&builder, codegen)?;

        // Add all resolvers to the map
        for resolver in &self.resolvers {
            self.add_resolver_to_map(&builder, map, resolver, codegen)?;
        }

        builder.build_return(Some(&map));
        Ok(function)
    }
}

struct Resolver<'ctx> {
    type_name: String,
    field_name: String,
    resolver_fn: FunctionValue<'ctx>,
}
