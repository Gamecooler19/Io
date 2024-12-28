use crate::{
    ast::ASTNode,
    visitor::{Visitor, Visitable},
    error::IoError,
    Result,
};

pub struct Validator {
    in_loop: bool,
    in_function: bool,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            in_loop: false,
            in_function: false,
        }
    }

    pub fn validate(&mut self, ast: &ASTNode) -> Result<()> {
        ast.accept(self)
    }
}

impl Visitor<()> for Validator {
    fn visit_program(&mut self, statements: &[ASTNode]) -> Result<()> {
        for stmt in statements {
            stmt.accept(self)?;
        }
        Ok(())
    }

    fn visit_function(&mut self, name: &str, params: &[Parameter], return_type: &Option<String>, body: &[ASTNode], is_async: bool) -> Result<()> {
        // Validate function name
        if !is_valid_identifier(name) {
            return Err(IoError::validation_error(
                format!("Invalid function name: {}", name)
            ));
        }

        let was_in_function = self.in_function;
        self.in_function = true;

        // Validate function body
        for stmt in body {
            stmt.accept(self)?;
        }

        self.in_function = was_in_function;
        Ok(())
    }

    // TODO: Implement other visitor methods with validation logic
}

fn is_valid_identifier(name: &str) -> bool {
    let first_char = match name.chars().next() {
        Some(c) => c.is_alphabetic() || c == '_',
        None => false,
    };

    first_char && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}
