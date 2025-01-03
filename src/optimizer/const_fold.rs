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

    fn visit_unary_operation(&mut self, operator: &str, operand: &ASTNode) -> Result<ASTNode> {
        self.fold_unary_operation(operator, operand)
    }

    fn visit_if_expression(&mut self, condition: &ASTNode, then_branch: &ASTNode, else_branch: &Option<Box<ASTNode>>) -> Result<ASTNode> {
        let folded_condition = self.fold(condition)?;
        let folded_then = self.fold(then_branch)?;
        let folded_else = if let Some(else_expr) = else_branch {
            Some(Box::new(self.fold(else_expr)?))
        } else {
            None
        };

        // Evaluate constant conditions
        match folded_condition {
            ASTNode::BooleanLiteral(true) => {
                self.changed = true;
                Ok(folded_then)
            }
            ASTNode::BooleanLiteral(false) if folded_else.is_some() => {
                self.changed = true;
                Ok(*folded_else.unwrap())
            }
            _ => Ok(ASTNode::IfExpression {
                condition: Box::new(folded_condition),
                then_branch: Box::new(folded_then),
                else_branch: folded_else,
            }),
        }
    }

    fn visit_block(&mut self, statements: &[ASTNode]) -> Result<ASTNode> {
        let mut folded_statements = Vec::new();
        let mut is_const = true;

        for stmt in statements {
            let folded = self.fold(stmt)?;
            is_const &= matches!(
                folded,
                ASTNode::IntegerLiteral(_) |
                ASTNode::FloatLiteral(_) |
                ASTNode::BooleanLiteral(_) |
                ASTNode::StringLiteral(_)
            );
            folded_statements.push(folded);
        }

        // If block contains only constant expressions, return the last one
        if is_const && !folded_statements.is_empty() {
            self.changed = true;
            Ok(folded_statements.pop().unwrap())
        } else {
            Ok(ASTNode::Block(folded_statements))
        }
    }

    fn visit_function_call(&mut self, name: &str, arguments: &[ASTNode]) -> Result<ASTNode> {
        let folded_args: Result<Vec<_>> = arguments.iter()
            .map(|arg| self.fold(arg))
            .collect();
        
        let folded_args = folded_args?;
        
        // Handle built-in function calls with constant arguments
        match name {
            "len" => {
                if let [ASTNode::ArrayLiteral(elements)] = folded_args.as_slice() {
                    self.changed = true;
                    return Ok(ASTNode::IntegerLiteral(elements.len() as i64));
                }
            }
            "typeof" => {
                if let [arg] = folded_args.as_slice() {
                    if let Some(type_name) = self.get_const_type(arg) {
                        self.changed = true;
                        return Ok(ASTNode::StringLiteral(type_name));
                    }
                }
            }
            "min" | "max" => {
                if folded_args.iter().all(|arg| matches!(arg, ASTNode::IntegerLiteral(_))) {
                    let numbers: Vec<i64> = folded_args.iter()
                        .map(|arg| match arg {
                            ASTNode::IntegerLiteral(n) => *n,
                            _ => unreachable!(),
                        })
                        .collect();
                    
                    self.changed = true;
                    return Ok(ASTNode::IntegerLiteral(
                        if name == "min" {
                            numbers.iter().min().copied().unwrap_or(0)
                        } else {
                            numbers.iter().max().copied().unwrap_or(0)
                        }
                    ));
                }
            }
            _ => {}
        }

        Ok(ASTNode::FunctionCall {
            name: name.to_string(),
            arguments: folded_args,
        })
    }

    fn visit_array_literal(&mut self, elements: &[ASTNode]) -> Result<ASTNode> {
        let folded_elements: Result<Vec<_>> = elements.iter()
            .map(|elem| self.fold(elem))
            .collect();
        
        Ok(ASTNode::ArrayLiteral(folded_elements?))
    }

    fn visit_array_index(&mut self, array: &ASTNode, index: &ASTNode) -> Result<ASTNode> {
        let folded_array = self.fold(array)?;
        let folded_index = self.fold(index)?;

        // Handle constant array indexing
        if let (ASTNode::ArrayLiteral(elements), ASTNode::IntegerLiteral(idx)) = (&folded_array, &folded_index) {
            if *idx >= 0 && (*idx as usize) < elements.len() {
                self.changed = true;
                return Ok(elements[*idx as usize].clone());
            }
        }

        Ok(ASTNode::ArrayIndex {
            array: Box::new(folded_array),
            index: Box::new(folded_index),
        })
    }

    fn visit_variable(&mut self, name: &str) -> Result<ASTNode> {
        Ok(ASTNode::Variable(name.to_string()))
    }

    fn visit_literal(&mut self, value: &ASTNode) -> Result<ASTNode> {
        Ok(value.clone())
    }
}

impl ConstantFolder {
    fn get_const_type(&self, node: &ASTNode) -> Option<String> {
        match node {
            ASTNode::IntegerLiteral(_) => Some("integer".to_string()),
            ASTNode::FloatLiteral(_) => Some("float".to_string()),
            ASTNode::BooleanLiteral(_) => Some("boolean".to_string()),
            ASTNode::StringLiteral(_) => Some("string".to_string()),
            ASTNode::ArrayLiteral(_) => Some("array".to_string()),
            _ => None,
        }
    }
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

    #[test]
    fn test_if_constant_folding() {
        let mut folder = ConstantFolder::new();
        
        let ast = ASTNode::IfExpression {
            condition: Box::new(ASTNode::BooleanLiteral(true)),
            then_branch: Box::new(ASTNode::IntegerLiteral(1)),
            else_branch: Some(Box::new(ASTNode::IntegerLiteral(2))),
        };
        
        let result = folder.fold(&ast).unwrap();
        assert_eq!(result, ASTNode::IntegerLiteral(1));
    }

    #[test]
    fn test_array_constant_folding() {
        let mut folder = ConstantFolder::new();
        
        let ast = ASTNode::ArrayIndex {
            array: Box::new(ASTNode::ArrayLiteral(vec![
                ASTNode::IntegerLiteral(1),
                ASTNode::IntegerLiteral(2),
            ])),
            index: Box::new(ASTNode::IntegerLiteral(0)),
        };
        
        let result = folder.fold(&ast).unwrap();
        assert_eq!(result, ASTNode::IntegerLiteral(1));
    }

    #[test]
    fn test_function_call_folding() {
        let mut folder = ConstantFolder::new();
        
        let ast = ASTNode::FunctionCall {
            name: "len".to_string(),
            arguments: vec![ASTNode::ArrayLiteral(vec![
                ASTNode::IntegerLiteral(1),
                ASTNode::IntegerLiteral(2),
            ])],
        };
        
        let result = folder.fold(&ast).unwrap();
        assert_eq!(result, ASTNode::IntegerLiteral(2));
    }
}
