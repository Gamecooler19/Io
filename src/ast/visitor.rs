use crate::{ast::{ASTNode, Parameter}, Result};

pub trait ASTVisitor {
    type Output;

    fn visit_node(&mut self, node: &ASTNode) -> Result<Self::Output>;
    fn visit_function(&mut self, name: &str, params: &[Parameter], return_type: &Option<String>, body: &[ASTNode], is_async: bool, location: &crate::ast::Location) -> Result<Self::Output>;
    fn visit_binary_op(&mut self, op: &crate::ast::BinaryOperator, left: &ASTNode, right: &ASTNode) -> Result<Self::Output>;
    fn visit_literal(&mut self, value: &str, ty: &str) -> Result<Self::Output>;
    fn visit_identifier(&mut self, name: &str) -> Result<Self::Output>;
    fn visit_call(&mut self, name: &str, args: &[ASTNode]) -> Result<Self::Output>;
}
