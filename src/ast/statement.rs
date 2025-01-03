use super::{Expression, Type};
use crate::ast::Literal;

#[derive(Debug, Clone)]
pub enum Statement {
    Expression(Expression),
    Return(Option<Expression>),
    Let {
        name: String,
        init: Expression,
        ty: Option<Type>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    For {
        init: Option<Box<Statement>>,
        cond: Option<Expression>,
        step: Option<Expression>,
        body: Vec<Statement>,
    },
    Block(Vec<Statement>),
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    Break,
    Continue,
    Loop {
        body: Vec<Statement>,
    },
    Match {
        expr: Expression,
        arms: Vec<MatchArm>,
    },
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Literal),
    Identifier(String),
    Struct {
        name: String,
        fields: Vec<(String, Pattern)>,
    },
    Array(Vec<Pattern>),
    Or(Vec<Pattern>),
    Wildcard,
}
