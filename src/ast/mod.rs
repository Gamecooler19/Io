use crate::error::IoError;
use crate::source_location::SourceLocation;
use crate::types::Type;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub file: String,
}

#[derive(Debug, Clone)]
pub enum ASTNode {
    IntLiteral {
        value: i64,
        location: Location,
    },
    FloatLiteral {
        value: f64,
        location: Location,
    },
    StringLiteral {
        value: String,
        location: Location,
    },
    BoolLiteral {
        value: bool,
        location: Location,
    },
    Identifier {
        name: String,
        location: Location,
    },
    BinaryOp {
        op: BinaryOperator,
        left: Box<ASTNode>,
        right: Box<ASTNode>,
        location: Location,
    },
    Call {
        name: String,
        args: Vec<ASTNode>,
        location: Location,
    },
    Function {
        name: String,
        params: Vec<Parameter>,
        return_type: Option<Type>,
        body: Box<ASTNode>,
        is_async: bool,
        location: Location,
    },
    Return {
        value: Option<Box<ASTNode>>,
        location: Location,
    },
    Block {
        statements: Vec<ASTNode>,
        location: Location,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub param_type: Type,
    pub location: Location,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    And,
    Or,
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinaryOperator::Add => write!(f, "+"),
            BinaryOperator::Subtract => write!(f, "-"),
            BinaryOperator::Multiply => write!(f, "*"),
            BinaryOperator::Divide => write!(f, "/"),
            BinaryOperator::Equal => write!(f, "=="),
            BinaryOperator::NotEqual => write!(f, "!="),
            BinaryOperator::LessThan => write!(f, "<"),
            BinaryOperator::LessThanEqual => write!(f, "<="),
            BinaryOperator::GreaterThan => write!(f, ">"),
            BinaryOperator::GreaterThanEqual => write!(f, ">="),
            BinaryOperator::And => write!(f, "&&"),
            BinaryOperator::Or => write!(f, "||"),
        }
    }
}

impl fmt::Display for ASTNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ASTNode::IntLiteral { value, .. } => write!(f, "IntLiteral({})", value),
            ASTNode::FloatLiteral { value, .. } => write!(f, "FloatLiteral({})", value),
            ASTNode::StringLiteral { value, .. } => write!(f, "StringLiteral({})", value),
            ASTNode::BoolLiteral { value, .. } => write!(f, "BoolLiteral({})", value),
            ASTNode::Identifier { name, .. } => write!(f, "Identifier({})", name),
            ASTNode::BinaryOp { op, .. } => write!(f, "BinaryOp({})", op),
            ASTNode::Call { name, .. } => write!(f, "Call({})", name),
            ASTNode::Function { name, .. } => write!(f, "Function({})", name),
            ASTNode::Return { .. } => write!(f, "Return"),
            ASTNode::Block { .. } => write!(f, "Block"),
        }
    }
}

pub trait ASTVisitor {
    type Output;
    fn visit_node(&mut self, node: &ASTNode) -> crate::Result<Self::Output>;
}

impl ASTNode {
    pub fn accept<V: ASTVisitor>(&self, visitor: &mut V) -> Result<(), IoError> {
        visitor.visit_node(self)
    }

    pub fn get_location(&self) -> &Location {
        match self {
            ASTNode::IntLiteral { location, .. } => location,
            ASTNode::FloatLiteral { location, .. } => location,
            ASTNode::StringLiteral { location, .. } => location,
            ASTNode::BoolLiteral { location, .. } => location,
            ASTNode::Identifier { location, .. } => location,
            ASTNode::BinaryOp { location, .. } => location,
            ASTNode::Call { location, .. } => location,
            ASTNode::Function { location, .. } => location,
            ASTNode::Return { location, .. } => location,
            ASTNode::Block { location, .. } => location,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_creation() {
        let func = ASTNode::Function {
            name: "test".to_string(),
            params: vec![],
            return_type: None,
            body: Box::new(ASTNode::Return {
                value: Some(Box::new(ASTNode::IntLiteral {
                    value: 42,
                    location: Location {
                        line: 1,
                        column: 1,
                        file: "test.rs".to_string(),
                    },
                })),
                location: Location {
                    line: 1,
                    column: 1,
                    file: "test.rs".to_string(),
                },
            }),
            is_async: false,
            location: Location {
                line: 1,
                column: 1,
                file: "test.rs".to_string(),
            },
        };

        assert!(matches!(func, ASTNode::Function { .. }));
    }

    #[test]
    fn test_pattern_matching() {
        let pattern = Pattern::Constructor {
            name: "Point".to_string(),
            fields: vec![
                Pattern::Variable("x".to_string()),
                Pattern::Variable("y".to_string()),
            ],
        };

        assert!(matches!(pattern, Pattern::Constructor { .. }));
    }

    #[test]
    fn test_control_flow_statements() {
        let if_stmt = ASTNode::If {
            condition: Box::new(Expression::BooleanLiteral(true)),
            then_branch: vec![ASTNode::Return(None)],
            else_branch: None,
        };

        let while_stmt = ASTNode::While {
            condition: Box::new(Expression::BooleanLiteral(true)),
            body: vec![ASTNode::Expression(Box::new(Expression::IntegerLiteral(1)))],
        };

        assert!(matches!(if_stmt, ASTNode::If { .. }));
        assert!(matches!(while_stmt, ASTNode::While { .. }));
    }
}
