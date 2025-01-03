use super::*;
use crate::Result;

pub trait ASTVisitor<T> {
    fn visit_program(&mut self, statements: &[ASTNode]) -> Result<T>;
    fn visit_function(&mut self, function: &Function) -> Result<T>;
    fn visit_statement(&mut self, statement: &Statement) -> Result<T>;
    fn visit_expression(&mut self, expression: &Expression) -> Result<T>;
    fn visit_binary_op(&mut self, op: &BinaryOperator, left: &Expression, right: &Expression) -> Result<T>;
    fn visit_if(&mut self, condition: &Expression, then_branch: &[Statement], else_branch: &Option<Vec<Statement>>) -> Result<T>;
    fn visit_while(&mut self, condition: &Expression, body: &[Statement]) -> Result<T>;
    fn visit_return(&mut self, value: &Option<Expression>) -> Result<T>;
}
