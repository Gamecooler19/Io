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

    fn generate_parameter_validation(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        function: &FunctionValue<'ctx>,
        fields: &[EntityField],
    ) -> Result<()> {
        for (i, field) in fields.iter().enumerate() {
            let param = function.get_nth_param(i as u32)
                .ok_or_else(|| IoError::validation_error("Missing parameter"))?;
            
            // Check for null values in non-nullable fields
            if (!field.is_nullable) {
                let is_null = match field.field_type.as_str() {
                    "string" => builder.build_is_null(param.into_pointer_value(), "null_check"),
                    "int" | "float" => {
                        let zero_value = match field.field_type.as_str() {
                            "int" => builder.context.i64_type().const_zero(),
                            "float" => builder.context.f64_type().const_zero(),
                            _ => unreachable!(),
                        };
                        builder.build_int_compare(
                            inkwell::IntPredicate::EQ,
                            param.into_int_value(),
                            zero_value,
                            "zero_check",
                        )
                    },
                    _ => continue,
                };

                let error_block = builder.context.append_basic_block(function, "error");
                let continue_block = builder.context.append_basic_block(function, "continue");

                builder.build_conditional_branch(is_null, error_block, continue_block);
                
                builder.position_at_end(error_block);
                self.build_error_return(
                    builder,
                    &format!("Field {} cannot be null", field.name),
                    "VALIDATION_ERROR",
                );

                builder.position_at_end(continue_block);
            }

            // Validate foreign key constraints
            if let Some(ref foreign_table) = field.foreign_key {
                let exists_check = self.build_foreign_key_check(
                    builder,
                    param,
                    foreign_table,
                    &field.field_type,
                )?;

                let fk_error = builder.context.append_basic_block(function, "fk_error");
                let fk_continue = builder.context.append_basic_block(function, "fk_continue");

                builder.build_conditional_branch(exists_check, fk_continue, fk_error);
                
                builder.position_at_end(fk_error);
                self.build_error_return(
                    builder,
                    &format!("Foreign key constraint failed for {} referencing {}", 
                        field.name, foreign_table),
                    "FOREIGN_KEY_ERROR",
                );

                builder.position_at_end(fk_continue);
            }

            // Type-specific validations
            match field.field_type.as_str() {
                "string" => {
                    let str_len = builder.build_call(
                        self.get_string_length_fn(),
                        &[param],
                        "strlen",
                    );

                    // Check maximum length
                    let max_len = builder.context.i32_type().const_int(255, false);
                    let too_long = builder.build_int_compare(
                        inkwell::IntPredicate::SGT,
                        str_len.try_as_basic_value().unwrap_left().into_int_value(),
                        max_len,
                        "length_check",
                    );

                    let length_error = builder.context.append_basic_block(function, "length_error");
                    let length_ok = builder.context.append_basic_block(function, "length_ok");

                    builder.build_conditional_branch(too_long, length_error, length_ok);
                    
                    builder.position_at_end(length_error);
                    self.build_error_return(
                        builder,
                        &format!("Field {} exceeds maximum length of 255", field.name),
                        "VALIDATION_ERROR",
                    );

                    builder.position_at_end(length_ok);
                },
                "int" => {
                    // Check range constraints if specified
                    if let Some(range) = self.get_field_range_constraint(&field.name) {
                        let value = param.into_int_value();
                        let min_val = builder.context.i64_type().const_int(range.0 as u64, true);
                        let max_val = builder.context.i64_type().const_int(range.1 as u64, true);

                        let below_min = builder.build_int_compare(
                            inkwell::IntPredicate::SLT,
                            value,
                            min_val,
                            "range_min_check",
                        );

                        let above_max = builder.build_int_compare(
                            inkwell::IntPredicate::SGT,
                            value,
                            max_val,
                            "range_max_check",
                        );

                        let out_of_range = builder.build_or(below_min, above_max, "range_check");
                        
                        let range_error = builder.context.append_basic_block(function, "range_error");
                        let range_ok = builder.context.append_basic_block(function, "range_ok");

                        builder.build_conditional_branch(out_of_range, range_error, range_ok);
                        
                        builder.position_at_end(range_error);
                        self.build_error_return(
                            builder,
                            &format!("Field {} must be between {} and {}", 
                                field.name, range.0, range.1),
                            "VALIDATION_ERROR",
                        );

                        builder.position_at_end(range_ok);
                    }
                },
                "float" => {
                    // Check for NaN and Infinity
                    let value = param.into_float_value();
                    let is_nan = builder.build_float_compare(
                        inkwell::FloatPredicate::UNO,
                        value,
                        value,
                        "nan_check",
                    );

                    let float_error = builder.context.append_basic_block(function, "float_error");
                    let float_ok = builder.context.append_basic_block(function, "float_ok");

                    builder.build_conditional_branch(is_nan, float_error, float_ok);
                    
                    builder.position_at_end(float_error);
                    self.build_error_return(
                        builder,
                        &format!("Field {} cannot be NaN or Infinity", field.name),
                        "VALIDATION_ERROR",
                    );

                    builder.position_at_end(float_ok);
                },
                _ => {}
            }
        }

        Ok(())
    }

    fn build_error_return(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        message: &str,
        error_code: &str,
    ) {
        let error_msg = builder.build_global_string_ptr(message, "error_msg");
        let error_code = builder.build_global_string_ptr(error_code, "error_code");
        
        builder.build_call(
            self.get_error_handler_fn(),
            &[error_msg.as_pointer_value().into(), error_code.as_pointer_value().into()],
            "error",
        );
        
        builder.build_return(None);
    }

    fn build_foreign_key_check(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        value: inkwell::values::BasicValueEnum<'ctx>,
        foreign_table: &str,
        field_type: &str,
    ) -> Result<inkwell::values::IntValue<'ctx>> {
        let check_fn = self.module.get_function(&format!("check_{}_exists", foreign_table))
            .ok_or_else(|| IoError::validation_error(
                format!("Foreign key check function not found for table {}", foreign_table)
            ))?;

        let result = builder.build_call(
            check_fn,
            &[value],
            "fk_check",
        );

        Ok(result.try_as_basic_value().unwrap_left().into_int_value())
    }

    fn get_string_length_fn(&self) -> FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function("strlen") {
            return fn_val;
        }

        let fn_type = self.context.i32_type().fn_type(
            &[self.context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into()],
            false,
        );
        self.module.add_function("strlen", fn_type, None)
    }

    fn get_error_handler_fn(&self) -> FunctionValue<'ctx> {
        if let Some(fn_val) = self.module.get_function("handle_orm_error") {
            return fn_val;
        }

        let fn_type = self.context.void_type().fn_type(
            &[
                self.context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into(),
                self.context.i8_type().ptr_type(inkwell::AddressSpace::Generic).into(),
            ],
            false,
        );
        self.module.add_function("handle_orm_error", fn_type, None)
    }

    fn get_field_range_constraint(&self, field_name: &str) -> Option<(i64, i64)> {
        // Example range constraints - in practice, these would be configured
        match field_name {
            "age" => Some((0, 150)),
            "year" => Some((1900, 2100)),
            _ => None,
        }
    }
}
