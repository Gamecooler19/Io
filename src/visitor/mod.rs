use crate::ast::{ASTNode, Expression, Function, Statement};
use crate::prelude::*;
use crate::Result;

pub trait Visitor<T> {
    fn visit_program(&mut self, statements: &[ASTNode]) -> Result<T>;
    fn visit_function(&mut self, function: &Function) -> Result<T>;
    fn visit_statement(&mut self, statement: &Statement) -> Result<T>;
    fn visit_expression(&mut self, expression: &Expression) -> Result<T>;

    // Control flow
    fn visit_loop(&mut self, condition: &ASTNode, body: &[ASTNode]) -> Result<T>;
    fn visit_break(&mut self) -> Result<T>;
    fn visit_continue(&mut self) -> Result<T>;
    fn visit_return(&mut self, value: &Option<ASTNode>) -> Result<T>;
    fn visit_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &[Statement],
        else_branch: &Option<Vec<Statement>>,
    ) -> Result<T>;

    // Expressions
    fn visit_binary_expr(&mut self, left: &ASTNode, operator: &str, right: &ASTNode) -> Result<T>;
    fn visit_unary_expr(&mut self, operator: &str, operand: &ASTNode) -> Result<T>;
    fn visit_call_expr(&mut self, callee: &ASTNode, arguments: &[ASTNode]) -> Result<T>;
    fn visit_assignment(&mut self, target: &ASTNode, value: &ASTNode) -> Result<T>;
    fn visit_identifier(&mut self, name: &str) -> Result<T>;
    fn visit_literal(&mut self, value: &Expression) -> Result<T>;

    // Declarations
    fn visit_variable_decl(
        &mut self,
        name: &str,
        type_name: &Option<String>,
        initializer: &Option<ASTNode>,
    ) -> Result<T>;
}

pub trait Visitable {
    fn accept<T>(&self, visitor: &mut dyn Visitor<T>) -> Result<T>;
}

impl Visitable for ASTNode {
    fn accept<T>(&self, visitor: &mut dyn Visitor<T>) -> Result<T> {
        match self {
            ASTNode::Program(statements) => visitor.visit_program(statements),
            ASTNode::Function {
                name,
                params,
                return_type,
                body,
                is_async,
            } => {
                let func = Function {
                    name: name.clone(),
                    parameters: params.clone(),
                    return_type: return_type.clone(),
                    body: body.clone(),
                    is_async: *is_async,
                };
                visitor.visit_function(&func)
            }
            ASTNode::Statement(stmt) => visitor.visit_statement(stmt),
            ASTNode::Expression(expr) => visitor.visit_expression(expr),
            ASTNode::BinaryOp { left, op, right } => {
                visitor.visit_binary_expr(left, &op.to_string(), right)
            }
            ASTNode::Call { callee, args } => visitor.visit_call_expr(callee, args),
            ASTNode::If {
                condition,
                then_branch,
                else_branch,
            } => visitor.visit_if(condition, then_branch, else_branch),
            ASTNode::Assignment { target, value } => visitor.visit_assignment(target, value),
            ASTNode::Identifier(name) => visitor.visit_identifier(name),
            ASTNode::Literal(value) => visitor.visit_literal(&Expression::Literal(value.clone())),
            _ => Err(crate::error::IoError::runtime_error(
                "Unsupported node type",
            )),
        }
    }
}

impl Visitor for SomeVisitor {
    fn visit_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &[ASTNode],
        else_branch: &Option<Vec<ASTNode>>,
    ) -> Result<()> {
        // ...existing code...
    }

    fn visit_call_expr(&mut self, name: &str, args: &[ASTNode]) -> Result<()> {
        // ...existing code...
    }

    // ...existing methods...
}
