use crate::{
    ast::ASTNode,
    visitor::{Visitor, Visitable},
    Result,
};

pub struct ConstantFolder {
    changed: bool,
}

impl ConstantFolder {
    pub fn new() -> Self {
        Self { changed: false }
    }

    pub fn fold(&mut self, ast: &ASTNode) -> Result<ASTNode> {
        ast.accept(self)
    }

    fn evaluate_binary_op(&self, op: &str, left: i64, right: i64) -> Option<i64> {
        match op {
            "+" => Some(left + right),
            "-" => Some(left - right),
            "*" => Some(left * right),
            "/" if right != 0 => Some(left / right),
            "%" if right != 0 => Some(left % right),
            _ => None,
        }
    }

    fn fold_binary_operation(&mut self, left: &ASTNode, op: &str, right: &ASTNode) -> Result<ASTNode> {
        let left_folded = self.fold(left)?;
        let right_folded = self.fold(right)?;

        match (&left_folded, &right_folded) {
            (ASTNode::IntegerLiteral(l), ASTNode::IntegerLiteral(r)) => {
                if let Some(result) = self.evaluate_binary_op(op, *l, *r) {
                    self.changed = true;
                    return Ok(ASTNode::IntegerLiteral(result));
                }
            }
            (ASTNode::FloatLiteral(l), ASTNode::FloatLiteral(r)) => {
                if let Some(result) = self.evaluate_float_op(op, *l, *r) {
                    self.changed = true;
                    return Ok(ASTNode::FloatLiteral(result));
                }
            }
            (ASTNode::BooleanLiteral(l), ASTNode::BooleanLiteral(r)) => {
                if let Some(result) = self.evaluate_bool_op(op, *l, *r) {
                    self.changed = true;
                    return Ok(ASTNode::BooleanLiteral(result));
                }
            }
            (ASTNode::StringLiteral(l), ASTNode::StringLiteral(r)) => {
                if op == "+" {
                    self.changed = true;
                    return Ok(ASTNode::StringLiteral(format!("{}{}", l, r)));
                }
            }
            _ => {}
        }

        Ok(ASTNode::BinaryOperation {
            left: Box::new(left_folded),
            operator: op.to_string(),
            right: Box::new(right_folded),
        })
    }

    fn evaluate_float_op(&self, op: &str, left: f64, right: f64) -> Option<f64> {
        match op {
            "+" => Some(left + right),
            "-" => Some(left - right),
            "*" => Some(left * right),
            "/" if right != 0.0 => Some(left / right),
            "%" if right != 0.0 => Some(left % right),
            _ => None,
        }
    }

    fn evaluate_bool_op(&self, op: &str, left: bool, right: bool) -> Option<bool> {
        match op {
            "&&" => Some(left && right),
            "||" => Some(left || right),
            "==" => Some(left == right),
            "!=" => Some(left != right),
            _ => None,
        }
    }

    fn fold_unary_operation(&mut self, op: &str, operand: &ASTNode) -> Result<ASTNode> {
        let folded_operand = self.fold(operand)?;

        match (&folded_operand, op) {
            (ASTNode::IntegerLiteral(v), "-") => {
                self.changed = true;
                Ok(ASTNode::IntegerLiteral(-v))
            }
            (ASTNode::FloatLiteral(v), "-") => {
                self.changed = true;
                Ok(ASTNode::FloatLiteral(-v))
            }
            (ASTNode::BooleanLiteral(v), "!") => {
                self.changed = true;
                Ok(ASTNode::BooleanLiteral(!v))
            }
            _ => Ok(ASTNode::UnaryOperation {
                operator: op.to_string(),
                operand: Box::new(folded_operand),
            }),
        }
    }
}

impl Visitor<ASTNode> for ConstantFolder {
    fn visit_binary_operation(&mut self, left: &ASTNode, operator: &str, right: &ASTNode) -> Result<ASTNode> {
        self.fold_binary_operation(left, operator, right)
    }

    // Implement other visitor methods...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_folding() {
        let mut folder = ConstantFolder::new();
        
        // Test integer operations
        let ast = ASTNode::BinaryOperation {
            left: Box::new(ASTNode::IntegerLiteral(2)),
            operator: "+".to_string(),
            right: Box::new(ASTNode::IntegerLiteral(3)),
        };
        
        let result = folder.fold(&ast).unwrap();
        assert_eq!(result, ASTNode::IntegerLiteral(5));
        
        // Test boolean operations
        let ast = ASTNode::BinaryOperation {
            left: Box::new(ASTNode::BooleanLiteral(true)),
            operator: "&&".to_string(),
            right: Box::new(ASTNode::BooleanLiteral(false)),
        };
        
        let result = folder.fold(&ast).unwrap();
        assert_eq!(result, ASTNode::BooleanLiteral(false));
    }
}
