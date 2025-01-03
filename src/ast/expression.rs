use super::{BinaryOperator, Literal};

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        arguments: Vec<Expression>,
    },
    ArrayAccess {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    StructAccess {
        object: Box<Expression>,
        field: String,
    },
    Lambda {
        params: Vec<String>,
        body: Box<Expression>,
    },
    Await(Box<Expression>),
}

#[derive(Debug, Clone)]
pub enum UnaryOperator {
    Negate,
    Not,
}
