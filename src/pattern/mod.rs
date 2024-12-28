use crate::{
    ast::{Pattern, ASTNode},
    types::Type,
    error::IoError,
    symbol_table::{Scope, Symbol},
    Result,
};

#[derive(Debug)]
pub struct PatternMatcher {
    scope: Box<Scope>,
}

impl PatternMatcher {
    pub fn new(parent_scope: Box<Scope>) -> Self {
        Self {
            scope: Box::new(Scope::with_parent(parent_scope)),
        }
    }

    pub fn match_pattern(&mut self, pattern: &Pattern, value: &ASTNode, value_type: &Type) -> Result<()> {
        match pattern {
            Pattern::Literal(lit) => self.match_literal(lit, value),
            Pattern::Variable(name) => self.bind_variable(name, value, value_type),
            Pattern::Wildcard => Ok(()), // Always matches
            Pattern::Constructor { name, fields } => self.match_constructor(name, fields, value),
        }
    }

    fn match_literal(&self, lit: &ASTNode, value: &ASTNode) -> Result<()> {
        match (lit, value) {
            (ASTNode::IntegerLiteral(l), ASTNode::IntegerLiteral(r)) if l == r => Ok(()),
            (ASTNode::StringLiteral(l), ASTNode::StringLiteral(r)) if l == r => Ok(()),
            (ASTNode::BooleanLiteral(l), ASTNode::BooleanLiteral(r)) if l == r => Ok(()),
            _ => Err(IoError::runtime_error("Pattern match failed")),
        }
    }

    fn bind_variable(&mut self, name: &str, value: &ASTNode, value_type: &Type) -> Result<()> {
        self.scope.define(
            name.to_string(),
            Symbol::Variable {
                name: name.to_string(),
                type_name: value_type.to_string(),
                mutable: false,
            },
        )
    }

    fn match_constructor(&mut self, name: &str, fields: &[Pattern], value: &ASTNode) -> Result<()> {
        // Implement constructor pattern matching
        match value {
            ASTNode::CallExpression { callee, arguments } => {
                if let ASTNode::Identifier(callee_name) = &**callee {
                    if callee_name == name && arguments.len() == fields.len() {
                        // Match each field with corresponding argument
                        for (field, arg) in fields.iter().zip(arguments) {
                            self.match_pattern(field, arg, &Type::Unit)?; 
                            // TODO: Get proper type
                        }
                        Ok(())
                    } else {
                        Err(IoError::runtime_error("Constructor pattern mismatch"))
                    }
                } else {
                    Err(IoError::runtime_error("Expected constructor"))
                }
            }
            _ => Err(IoError::runtime_error("Expected constructor call")),
        }
    }
}
