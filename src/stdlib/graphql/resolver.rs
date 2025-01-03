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

    fn generate_resolver_map_initialization(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>> {
        // Create map type
        let map_struct = codegen.get_type("GraphQLResolverMap")?;
        let map_ptr = builder.build_alloca(map_struct.into_struct_type(), "resolver_map");
        
        // Initialize fields
        let type_map = builder.build_call(
            codegen.module.get_function("create_type_map").ok_or_else(|| 
                IoError::runtime_error("Type map creation function not found"))?,
            &[],
            "type_map",
        ).try_as_basic_value().left().unwrap();

        let field_map = builder.build_call(
            codegen.module.get_function("create_field_map").ok_or_else(|| 
                IoError::runtime_error("Field map creation function not found"))?,
            &[],
            "field_map",
        ).try_as_basic_value().left().unwrap();

        // Store fields in map
        builder.build_struct_gep(map_ptr, 0, "type_map_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get type map pointer"))?;
        builder.build_store(map_ptr, type_map);

        builder.build_struct_gep(map_ptr, 1, "field_map_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get field map pointer"))?;
        builder.build_store(map_ptr, field_map);

        Ok(builder.build_load(map_ptr, "resolver_map"))
    }

    fn add_resolver_to_map(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        map: inkwell::values::BasicValueEnum<'ctx>,
        resolver: &Resolver<'ctx>,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        // Create resolver info
        let resolver_info = self.create_resolver_info(builder, resolver, codegen)?;
        
        // Get map insertion function
        let insert_fn = codegen.module.get_function("insert_resolver")
            .ok_or_else(|| IoError::runtime_error("Resolver insertion function not found"))?;
        
        // Create type and field name strings
        let type_name = builder.build_global_string_ptr(&resolver.type_name, "type_name");
        let field_name = builder.build_global_string_ptr(&resolver.field_name, "field_name");
        
        // Insert resolver into map
        builder.build_call(
            insert_fn,
            &[
                map.into(),
                type_name.as_pointer_value().into(),
                field_name.as_pointer_value().into(),
                resolver_info.into(),
            ],
            "insert_result",
        );

        // Add error checking
        self.build_insertion_error_check(builder, codegen)?;

        Ok(())
    }

    fn create_resolver_info(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        resolver: &Resolver<'ctx>,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>> {
        // Create resolver info struct
        let info_type = codegen.get_type("ResolverInfo")?;
        let info_ptr = builder.build_alloca(info_type.into_struct_type(), "resolver_info");

        // Set resolver function
        builder.build_struct_gep(info_ptr, 0, "fn_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get function pointer"))?;
        builder.build_store(info_ptr, resolver.resolver_fn.as_global_value().as_pointer_value());

        // Set resolver metadata
        let metadata = self.create_resolver_metadata(builder, resolver, codegen)?;
        builder.build_struct_gep(info_ptr, 1, "metadata_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get metadata pointer"))?;
        builder.build_store(info_ptr, metadata);

        Ok(builder.build_load(info_ptr, "resolver_info"))
    }

    fn create_resolver_metadata(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        resolver: &Resolver<'ctx>,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>> {
        // Create metadata struct
        let metadata_type = codegen.get_type("ResolverMetadata")?;
        let metadata_ptr = builder.build_alloca(metadata_type.into_struct_type(), "metadata");

        // Set resolver name
        let name = builder.build_global_string_ptr(
            &format!("{}.{}", resolver.type_name, resolver.field_name),
            "resolver_name",
        );
        builder.build_struct_gep(metadata_ptr, 0, "name_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get name pointer"))?;
        builder.build_store(metadata_ptr, name.as_pointer_value());

        // Set resolver flags
        let flags = self.get_resolver_flags(resolver);
        builder.build_struct_gep(metadata_ptr, 1, "flags_ptr")
            .map_err(|_| IoError::runtime_error("Failed to get flags pointer"))?;
        builder.build_store(
            metadata_ptr,
            codegen.context.i32_type().const_int(flags as u64, false),
        );

        Ok(builder.build_load(metadata_ptr, "metadata"))
    }

    fn build_insertion_error_check(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        let error_block = codegen.context.append_basic_block(
            builder.get_insert_block().unwrap().get_parent().unwrap(),
            "insertion_error",
        );
        let continue_block = codegen.context.append_basic_block(
            builder.get_insert_block().unwrap().get_parent().unwrap(),
            "continue",
        );

        // Check insertion result
        let result = builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            builder.build_load(
                builder.build_alloca(codegen.context.i32_type(), "result"),
                "result",
            ).into_int_value(),
            codegen.context.i32_type().const_zero(),
            "is_error",
        );

        builder.build_conditional_branch(result, error_block, continue_block);

        // Build error handling
        builder.position_at_end(error_block);
        builder.build_call(
            codegen.module.get_function("handle_resolver_error")
                .ok_or_else(|| IoError::runtime_error("Error handler not found"))?,
            &[],
            "error",
        );
        builder.build_unreachable();

        builder.position_at_end(continue_block);
        Ok(())
    }

    fn get_resolver_flags(&self, resolver: &Resolver<'ctx>) -> u32 {
        let mut flags = 0;
        // Add flags based on resolver properties
        if resolver.type_name == "Query" || resolver.type_name == "Mutation" {
            flags |= 1; // Root resolver flag
        }
        if resolver
    }
}

struct Resolver<'ctx> {
    type_name: String,
    field_name: String,
    resolver_fn: FunctionValue<'ctx>,
}
