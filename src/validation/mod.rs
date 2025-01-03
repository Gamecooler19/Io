use crate::{
    ast::{ASTNode, Expression, Function, Statement},
    visitor::{Visitable, Visitor},
    IoError, Result,
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

    fn visit_function(&mut self, function: &Function) -> Result<()> {
        if !is_valid_identifier(&function.name) {
            return Err(IoError::validation_error(format!(
                "Invalid function name: {}",
                function.name
            )));
        }

        let was_in_function = self.in_function;
        self.in_function = true;

        for stmt in &function.body {
            stmt.accept(self)?;
        }

        self.in_function = was_in_function;
        Ok(())
    }

    fn visit_loop(&mut self, condition: &ASTNode, body: &[ASTNode]) -> Result<()> {
        let was_in_loop = self.in_loop;
        self.in_loop = true;

        condition.accept(self)?;

        for stmt in body {
            stmt.accept(self)?;
        }

        self.in_loop = was_in_loop;
        Ok(())
    }

    fn visit_break(&mut self) -> Result<()> {
        if !self.in_loop {
            return Err(IoError::validation_error("Break statement outside of loop"));
        }
        Ok(())
    }

    fn visit_continue(&mut self) -> Result<()> {
        if !self.in_loop {
            return Err(IoError::validation_error(
                "Continue statement outside of loop",
            ));
        }
        Ok(())
    }

    fn visit_return(&mut self, value: &Option<ASTNode>) -> Result<()> {
        if !self.in_function {
            return Err(IoError::validation_error(
                "Return statement outside of function",
            ));
        }

        if let Some(expr) = value {
            expr.accept(self)?;
        }
        Ok(())
    }

    fn visit_variable_decl(
        &mut self,
        name: &str,
        type_name: &Option<String>,
        initializer: &Option<ASTNode>,
    ) -> Result<()> {
        if !is_valid_identifier(name) {
            return Err(IoError::validation_error(format!(
                "Invalid variable name: {}",
                name
            )));
        }

        if let Some(init) = initializer {
            init.accept(self)?;
        }
        Ok(())
    }

    fn visit_call_expr(&mut self, callee: &ASTNode, arguments: &[ASTNode]) -> Result<()> {
        callee.accept(self)?;

        for arg in arguments {
            arg.accept(self)?;
        }
        Ok(())
    }

    fn visit_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &[ASTNode],
        else_branch: &Option<Vec<ASTNode>>,
    ) -> Result<()> {
        condition.accept(self)?;

        for stmt in then_branch {
            stmt.accept(self)?;
        }

        if let Some(else_stmts) = else_branch {
            for stmt in else_stmts {
                stmt.accept(self)?;
            }
        }
        Ok(())
    }

    fn visit_assignment(&mut self, target: &ASTNode, value: &ASTNode) -> Result<()> {
        match target {
            ASTNode::Identifier(_) | ASTNode::MemberAccess { .. } => {
                target.accept(self)?;
                value.accept(self)
            }
            _ => Err(IoError::validation_error("Invalid assignment target")),
        }
    }

    fn visit_statement(&mut self, _statement: &Statement) -> Result<()> {
        Ok(())
    }

    fn visit_expression(&mut self, _expression: &Expression) -> Result<()> {
        Ok(())
    }

    fn visit_binary_expr(
        &mut self,
        _left: &Expression,
        _operator: &str,
        _right: &Expression,
    ) -> Result<()> {
        Ok(())
    }

    fn visit_unary_expr(&mut self, _operator: &str, _operand: &Expression) -> Result<()> {
        Ok(())
    }

    fn visit_identifier(&mut self, _name: &str) -> Result<()> {
        Ok(())
    }

    fn visit_literal(&mut self, _value: &Expression) -> Result<()> {
        Ok(())
    }
}

fn is_valid_identifier(name: &str) -> bool {
    let first_char = match name.chars().next() {
        Some(c) => c.is_alphabetic() || c == '_',
        None => false,
    };

    first_char && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn validate_node(&self, node: &ASTNode) -> Result<()> {
    match node {
        ASTNode::Identifier(_) | ASTNode::MemberAccess { .. } => {
            // ...existing code...
        } // ...existing match arms...
    }
}
