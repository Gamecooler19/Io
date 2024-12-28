use std::collections::{HashMap, HashSet};
use crate::{Result, error::IoError};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Role {
    name: String,
    permissions: HashSet<String>,
    parent_roles: HashSet<String>,
}

#[derive(Debug)]
pub struct RBACManager<'ctx> {
    context: &'ctx inkwell::context::Context,
    roles: HashMap<String, Role>,
    role_hierarchy: HashMap<String, HashSet<String>>,
    permission_checks: HashMap<String, inkwell::values::FunctionValue<'ctx>>,
}

impl<'ctx> RBACManager<'ctx> {
    pub fn new(context: &'ctx inkwell::context::Context) -> Self {
        Self {
            context,
            roles: HashMap::new(),
            role_hierarchy: HashMap::new(),
            permission_checks: HashMap::new(),
        }
    }

    pub fn add_role(&mut self, name: &str, permissions: HashSet<String>, parent_roles: HashSet<String>) -> Result<()> {
        if self.roles.contains_key(name) {
            return Err(IoError::validation_error(format!("Role {} already exists", name)));
        }

        // Validate parent roles exist
        for parent in &parent_roles {
            if !self.roles.contains_key(parent) {
                return Err(IoError::validation_error(format!("Parent role {} does not exist", parent)));
            }
        }

        let role = Role {
            name: name.to_string(),
            permissions,
            parent_roles,
        };

        self.roles.insert(name.to_string(), role);
        self.update_role_hierarchy(name)?;
        Ok(())
    }

    fn update_role_hierarchy(&mut self, role_name: &str) -> Result<()> {
        let mut inherited_roles = HashSet::new();
        let mut to_process = vec![role_name.to_string()];

        while let Some(current_role) = to_process.pop() {
            if let Some(role) = self.roles.get(&current_role) {
                for parent in &role.parent_roles {
                    if inherited_roles.insert(parent.clone()) {
                        to_process.push(parent.clone());
                    }
                }
            }
        }

        self.role_hierarchy.insert(role_name.to_string(), inherited_roles);
        Ok(())
    }

    pub fn generate_check_permission(
        &self,
        codegen: &mut crate::codegen::llvm::LLVMCodeGen<'ctx>,
        permission: &str,
    ) -> Result<inkwell::values::FunctionValue<'ctx>> {
        let fn_type = codegen.context.bool_type().fn_type(&[
            codegen.get_type("Context")?.into(),
            codegen.string_type().into(), // Role name
        ], false);

        let function = codegen.module.add_function(
            &format!("check_permission_{}", permission.replace('.', "_")),
            fn_type,
            None,
        );

        let builder = codegen.context.create_builder();
        let entry = codegen.context.append_basic_block(function, "entry");
        builder.position_at_end(entry);

        // Generate permission check logic
        self.generate_permission_check_logic(&builder, function, permission)?;

        Ok(function)
    }

    fn generate_permission_check_logic(
        &self,
        builder: &inkwell::builder::Builder<'ctx>,
        function: inkwell::values::FunctionValue<'ctx>,
        permission: &str,
    ) -> Result<()> {
        let context_param = function.get_nth_param(0).unwrap();
        let role_param = function.get_nth_param(1).unwrap();

        // Generate role hierarchy check
        let result = self.generate_role_hierarchy_check(builder, context_param, role_param, permission)?;
        builder.build_return(Some(&result));

        Ok(())
    }
}
