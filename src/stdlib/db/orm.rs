use crate::{error::IoError, Result};
use inkwell::values::FunctionValue;
use std::collections::HashMap;

#[derive(Debug)]
pub struct EntityField {
    name: String,
    field_type: String,
    is_primary: bool,
    is_nullable: bool,
    foreign_key: Option<String>,
}

pub struct OrmModule<'ctx> {
    context: &'ctx inkwell::context::Context,
    entity_type: inkwell::types::StructType<'ctx>,
    query_builder_type: inkwell::types::StructType<'ctx>,
    entities: HashMap<String, Vec<EntityField>>,
}

impl<'ctx> OrmModule<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Result<Self> {
        let entity_type = context.opaque_struct_type("Entity");
        let query_builder_type = context.opaque_struct_type("QueryBuilder");

        Ok(Self {
            context,
            entity_type,
            query_builder_type,
            entities: HashMap::new(),
        })
    }

    pub fn register_entity(
        &mut self,
        name: &str,
        fields: Vec<EntityField>,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        // Generate entity struct type
        let field_types: Vec<_> = fields.iter()
            .map(|field| self.get_field_type(codegen, &field.field_type))
            .collect::<Result<_>>()?;

        let entity_struct = self.context.struct_type(&field_types, false);
        codegen.add_type(&format!("Entity_{}", name), entity_struct);

        // Generate CRUD operations
        self.generate_crud_operations(name, &fields, codegen)?;
        
        self.entities.insert(name.to_string(), fields);
        Ok(())
    }

    fn generate_crud_operations(
        &self,
        entity_name: &str,
        fields: &[EntityField],
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        // Create
        self.generate_create_method(entity_name, fields, codegen)?;
        // Read
        self.generate_find_method(entity_name, fields, codegen)?;
        // Update
        self.generate_update_method(entity_name, fields, codegen)?;
        // Delete
        self.generate_delete_method(entity_name, fields, codegen)?;
        
        Ok(())
    }

    fn generate_create_method(
        &self,
        entity_name: &str,
        fields: &[EntityField],
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
    ) -> Result<()> {
        let fn_name = format!("create_{}", entity_name.to_lowercase());
        let entity_type = codegen.get_type(&format!("Entity_{}", entity_name))?;
        
        let fn_type = entity_type.fn_type(
            &fields.iter()
                .map(|f| self.get_field_type(codegen, &f.field_type))
                .collect::<Result<Vec<_>>>()?
                .as_slice(),
            false
        );

        let function = codegen.module.add_function(&fn_name, fn_type, None);
        let builder = codegen.context.create_builder();
        let entry = codegen.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        // Generate SQL insert statement
        let insert_sql = self.generate_insert_statement(entity_name, fields);
        
        // Add parameter validation and error handling
        self.generate_parameter_validation(&builder, &function, fields)?;

        Ok(())
    }

    fn get_field_type(
        &self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        field_type: &str,
    ) -> Result<inkwell::types::BasicTypeEnum<'ctx>> {
        match field_type {
            "int" => Ok(codegen.context.i64_type().into()),
            "float" => Ok(codegen.context.f64_type().into()),
            "string" => Ok(codegen.string_type().into()),
            "bool" => Ok(codegen.context.bool_type().into()),
            _ => Err(IoError::type_error(format!("Unsupported field type: {}", field_type))),
        }
    }

    fn generate_insert_statement(&self, entity_name: &str, fields: &[EntityField]) -> String {
        let columns = fields.iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let values = fields.iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");

        format!("INSERT INTO {} ({}) VALUES ({})", entity_name, columns, values)
    }
}
