use super::{Expression, Statement};

#[derive(Debug, Clone)]
pub enum ASTNode {
    Program(Vec<ASTNode>),
    Function {
        name: String,
        params: Vec<super::Parameter>,
        return_type: Option<String>,
        body: Vec<ASTNode>,
        is_async: bool,
    },
    Statement(Statement),
    Expression(Expression),
    Block(Vec<ASTNode>),
    Call {
        name: String,
        args: Vec<ASTNode>,
    },
    If {
        condition: Box<ASTNode>,
        then_branch: Vec<ASTNode>,
        else_branch: Option<Vec<ASTNode>>,
    },
    While {
        condition: Box<ASTNode>,
        body: Vec<ASTNode>,
    },
    Return(Option<Box<ASTNode>>),
    Let {
        name: String,
        value: Box<ASTNode>,
    },
    Identifier(String),
    IntegerLiteral(i64),
    BinaryOp {
        op: String,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
    },
    Break,
    Continue,
    IntLiteral {
        value: i32,
    }, // Added IntLiteral variant
    FloatLiteral {
        value: f32,
    }, // Added FloatLiteral variant
    StringLiteral {
        value: String,
    }, // Added StringLiteral variant
    BoolLiteral {
        value: bool,
    }, // Added BoolLiteral variant
    Assignment {
        target: String,
        value: Box<ASTNode>,
    }, // Added Assignment variant
    MemberAccess {
        object: Box<ASTNode>,
        member: String,
    }, // Added MemberAccess variant
}
