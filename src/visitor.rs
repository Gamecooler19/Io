use crate::{
    ast::ASTNode,
    error::IoError,
    Result,
};

pub trait Visitor<T> {
    fn visit_program(&mut self, statements: &[ASTNode]) -> Result<T>;
    fn visit_function(&mut self, name: &str, params: &[Parameter], return_type: &Option<String>, body: &[ASTNode], is_async: bool) -> Result<T>;
    fn visit_variable_declaration(&mut self, name: &str, type_annotation: &Option<String>, value: &ASTNode) -> Result<T>;
    fn visit_binary_operation(&mut self, left: &ASTNode, operator: &str, right: &ASTNode) -> Result<T>;
    fn visit_identifier(&mut self, name: &str) -> Result<T>;
    fn visit_literal(&mut self, value: &ASTNode) -> Result<T>;
    fn visit_return(&mut self, value: &Option<Box<ASTNode>>) -> Result<T>;
    fn visit_if(&mut self, condition: &ASTNode, then_branch: &[ASTNode], else_branch: &Option<Vec<ASTNode>>) -> Result<T>;
    fn visit_while(&mut self, condition: &ASTNode, body: &[ASTNode]) -> Result<T>;
    fn visit_await(&mut self, expression: &ASTNode) -> Result<T>;
}

pub trait Visitable {
    fn accept<T>(&self, visitor: &mut dyn Visitor<T>) -> Result<T>;
}

impl Visitable for ASTNode {
    fn accept<T>(&self, visitor: &mut dyn Visitor<T>) -> Result<T> {
        match self {
            ASTNode::Program(statements) => visitor.visit_program(statements),
            ASTNode::Function { name, params, return_type, body, is_async } => {
                visitor.visit_function(name, params, return_type, body, *is_async)
            },
            // ... implement for other variants
            _ => Err(IoError::runtime_error("Unsupported AST node for visitor")),
        }
    }
}
