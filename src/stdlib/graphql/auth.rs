use std::collections::HashMap;
use crate::{Result, error::IoError};

#[derive(Clone, Debug)]
pub enum Permission {
    Read,
    Write,
    Admin,
}

#[derive(Clone, Debug)]
pub struct FieldAuth {
    permissions: Vec<Permission>,
    custom_checks: Vec<String>,
}

pub struct AuthorizationManager<'ctx> {
    context: &'ctx inkwell::context::Context,
    field_rules: HashMap<String, FieldAuth>,
    role_fn: Option<inkwell::values::FunctionValue<'ctx>>,
}

impl<'ctx> AuthorizationManager<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Self {
        Self {
            context,
            field_rules: HashMap::new(),
            role_fn: None,
        }
    }

    pub fn add_field_rule(
        &mut self,
        field_path: &str,
        permissions: Vec<Permission>,
        custom_checks: Vec<String>,
    ) -> Result<()> {
        self.field_rules.insert(
            field_path.to_string(),
            FieldAuth {
                permissions,
                custom_checks,
            },
        );
        Ok(())
    }

    pub fn generate_auth_check(
        &self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        field_path: &str,
    ) -> Result<inkwell::values::FunctionValue<'ctx>> {
        let fn_type = codegen.context.bool_type().fn_type(&[
            codegen.get_type("Context")?.into(),
            codegen.string_type().into(), // Field path
        ], false);

        let function = codegen.module.add_function(
            &format!("check_auth_{}", field_path.replace('.', "_")),
            fn_type,
            None,
        );

        let builder = codegen.context.create_builder();
        let entry = codegen.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        // Generate permission checks
        if let Some(auth) = self.field_rules.get(field_path) {
            self.generate_permission_checks(&builder, function, auth)?;
        }

        Ok(function)
    }

    fn generate_permission_checks(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        function: inkwell::values::FunctionValue<'ctx>,
        auth: &FieldAuth,
    ) -> Result<()> {
        let context_param = function.get_nth_param(0).unwrap();
        let field_path = function.get_nth_param(1).unwrap();

        // Generate checks for each permission
        for permission in &auth.permissions {
            self.generate_single_permission_check(builder, context_param, permission)?;
        }

        // Generate custom check calls
        for check in &auth.custom_checks {
            self.generate_custom_check(builder, context_param, check)?;
        }

        Ok(())
    }
}
