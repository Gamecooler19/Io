use std::collections::HashMap;
use crate::{
    ast::{ASTNode, ASTVisitor, BinaryOperator},
    error::IoError,
    Result,
};

#[derive(Debug, Clone)]
enum ConstValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
}

pub struct ConstantPropagator {
    constants: HashMap<String, ConstValue>,
    modified: bool,
}

impl ConstantPropagator {
    pub fn new() -> Self {
        Self {
            constants: HashMap::new(),
            modified: false,
        }
    }

    pub fn optimize(&mut self, node: &ASTNode) -> Result<ASTNode> {
        self.visit_node(node)
    }

    fn evaluate_binary_const(&self, op: &BinaryOperator, left: &ConstValue, right: &ConstValue) -> Option<ConstValue> {
        match (left, right, op) {
            // Integer operations
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::Add) => Some(ConstValue::Int(l + r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::Subtract) => Some(ConstValue::Int(l - r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::Multiply) => Some(ConstValue::Int(l * r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::Divide) if *r != 0 => Some(ConstValue::Int(l / r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::Modulo) if *r != 0 => Some(ConstValue::Int(l % r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::BitwiseAnd) => Some(ConstValue::Int(l & r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::BitwiseOr) => Some(ConstValue::Int(l | r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::BitwiseXor) => Some(ConstValue::Int(l ^ r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::LeftShift) => Some(ConstValue::Int(l << r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::RightShift) => Some(ConstValue::Int(l >> r)),

            // Float operations
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::Add) => Some(ConstValue::Float(l + r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::Subtract) => Some(ConstValue::Float(l - r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::Multiply) => Some(ConstValue::Float(l * r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::Divide) if *r != 0.0 => Some(ConstValue::Float(l / r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::Power) => Some(ConstValue::Float(l.powf(*r))),

            // String operations
            (ConstValue::String(l), ConstValue::String(r), BinaryOperator::Add) => Some(ConstValue::String(format!("{}{}", l, r))),
            
            // Boolean operations
            (ConstValue::Bool(l), ConstValue::Bool(r), BinaryOperator::And) => Some(ConstValue::Bool(*l && *r)),
            (ConstValue::Bool(l), ConstValue::Bool(r), BinaryOperator::Or) => Some(ConstValue::Bool(*l || *r)),
            (ConstValue::Bool(l), ConstValue::Bool(r), BinaryOperator::Xor) => Some(ConstValue::Bool(*l ^ *r)),

            // Comparison operations for integers
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::Equal) => Some(ConstValue::Bool(l == r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::NotEqual) => Some(ConstValue::Bool(l != r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::LessThan) => Some(ConstValue::Bool(l < r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::LessEqual) => Some(ConstValue::Bool(l <= r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::GreaterThan) => Some(ConstValue::Bool(l > r)),
            (ConstValue::Int(l), ConstValue::Int(r), BinaryOperator::GreaterEqual) => Some(ConstValue::Bool(l >= r)),

            // Comparison operations for floats
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::Equal) => Some(ConstValue::Bool(l == r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::NotEqual) => Some(ConstValue::Bool(l != r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::LessThan) => Some(ConstValue::Bool(l < r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::LessEqual) => Some(ConstValue::Bool(l <= r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::GreaterThan) => Some(ConstValue::Bool(l > r)),
            (ConstValue::Float(l), ConstValue::Float(r), BinaryOperator::GreaterEqual) => Some(ConstValue::Bool(l >= r)),

            // String comparison
            (ConstValue::String(l), ConstValue::String(r), BinaryOperator::Equal) => Some(ConstValue::Bool(l == r)),
            (ConstValue::String(l), ConstValue::String(r), BinaryOperator::NotEqual) => Some(ConstValue::Bool(l != r)),
            (ConstValue::String(l), ConstValue::String(r), BinaryOperator::LessThan) => Some(ConstValue::Bool(l < r)),
            (ConstValue::String(l), ConstValue::String(r), BinaryOperator::LessEqual) => Some(ConstValue::Bool(l <= r)),
            (ConstValue::String(l), ConstValue::String(r), BinaryOperator::GreaterThan) => Some(ConstValue::Bool(l > r)),
            (ConstValue::String(l), ConstValue::String(r), BinaryOperator::GreaterEqual) => Some(ConstValue::Bool(l >= r)),

            // Mixed numeric operations (int-float conversions)
            (ConstValue::Int(l), ConstValue::Float(r), op) => self.evaluate_binary_const(op, &ConstValue::Float(*l as f64), right),
            (ConstValue::Float(l), ConstValue::Int(r), op) => self.evaluate_binary_const(op, left, &ConstValue::Float(*r as f64)),

            _ => None,
        }
    }

    fn extract_constant(&self, node: &ASTNode) -> Option<ConstValue> {
        match node {
            ASTNode::IntegerLiteral(v) => Some(ConstValue::Int(*v)),
            ASTNode::FloatLiteral(v) => Some(ConstValue::Float(*v)),
            ASTNode::BooleanLiteral(v) => Some(ConstValue::Bool(*v)),
            ASTNode::StringLiteral(v) => Some(ConstValue::String(v.clone())),
            ASTNode::Identifier(name) => self.constants.get(name).cloned(),
            _ => None,
        }
    }

    fn const_value_to_ast(&self, value: &ConstValue) -> ASTNode {
        match value {
            ConstValue::Int(v) => ASTNode::IntegerLiteral(*v),
            ConstValue::Float(v) => ASTNode::FloatLiteral(*v),
            ConstValue::Bool(v) => ASTNode::BooleanLiteral(*v),
            ConstValue::String(v) => ASTNode::StringLiteral(v.clone()),
        }
    }
}

impl ASTVisitor for ConstantPropagator {
    type Output = ASTNode;

    fn visit_program(&mut self, nodes: &[ASTNode]) -> Result<Self::Output> {
        let mut optimized = Vec::with_capacity(nodes.len());
        for node in nodes {
            optimized.push(self.optimize(node)?);
        }
        Ok(ASTNode::Program(optimized))
    }

    fn visit_variable_declaration(&mut self, name: &str, type_annotation: &Option<String>, value: &ASTNode) -> Result<ASTNode> {
        let optimized_value = self.optimize(value)?;
        
        // If the value is constant, store it for propagation
        if let Some(const_value) = self.extract_constant(&optimized_value) {
            self.constants.insert(name.to_string(), const_value);
            self.modified = true;
        }

        Ok(ASTNode::VariableDeclaration {
            name: name.to_string(),
            type_annotation: type_annotation.clone(),
            value: Box::new(optimized_value),
            is_mutable: false,
        })
    }

    fn visit_identifier(&mut self, name: &str) -> Result<ASTNode> {
        // Replace identifier with constant value if available
        if let Some(value) = self.constants.get(name) {
            self.modified = true;
            Ok(self.const_value_to_ast(value))
        } else {
            Ok(ASTNode::Identifier(name.to_string()))
        }
    }

    fn visit_binary_operation(&mut self, left: &ASTNode, op: &BinaryOperator, right: &ASTNode) -> Result<ASTNode> {
        let left_opt = self.optimize(left)?;
        let right_opt = self.optimize(right)?;

        if let (Some(left_val), Some(right_val)) = (
            self.extract_constant(&left_opt),
            self.extract_constant(&right_opt)
        ) {
            if let Some(result) = self.evaluate_binary_const(op, &left_val, &right_val) {
                self.modified = true;
                return Ok(self.const_value_to_ast(&result));
            }
        }

        Ok(ASTNode::BinaryOperation {
            left: Box::new(left_opt),
            operator: op.clone(),
            right: Box::new(right_opt),
        })
    }

    fn visit_if(&mut self, condition: &ASTNode, then_branch: &[ASTNode], else_branch: &Option<Vec<ASTNode>>) -> Result<ASTNode> {
        let optimized_condition = self.optimize(condition)?;

        // If condition is constant, eliminate dead branch
        if let Some(ConstValue::Bool(cond_value)) = self.extract_constant(&optimized_condition) {
            self.modified = true;
            if cond_value {
                return Ok(ASTNode::Block(
                    then_branch.iter()
                        .map(|node| self.optimize(node))
                        .collect::<Result<_>>()?
                ));
            } else if let Some(else_nodes) = else_branch {
                return Ok(ASTNode::Block(
                    else_nodes.iter()
                        .map(|node| self.optimize(node))
                        .collect::<Result<_>>()?
                ));
            }
        }

        Ok(ASTNode::If {
            condition: Box::new(optimized_condition),
            then_branch: then_branch.iter()
                .map(|node| self.optimize(node))
                .collect::<Result<_>>()?,
            else_branch: if let Some(else_nodes) = else_branch {
                Some(else_nodes.iter()
                    .map(|node| self.optimize(node))
                    .collect::<Result<_>>()?)
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_propagation() {
        let mut propagator = ConstantPropagator::new();
        
        // Test basic propagation
        let ast = ASTNode::Block(vec![
            ASTNode::VariableDeclaration {
                name: "x".to_string(),
                type_annotation: None,
                value: Box::new(ASTNode::IntegerLiteral(42)),
                is_mutable: false,
            },
            ASTNode::BinaryOperation {
                left: Box::new(ASTNode::Identifier("x".to_string())),
                operator: BinaryOperator::Add,
                right: Box::new(ASTNode::IntegerLiteral(10)),
            },
        ]);

        let result = propagator.optimize(&ast).unwrap();
        assert!(propagator.modified);

        // Verify the result
        if let ASTNode::Block(nodes) = result {
            if let ASTNode::BinaryOperation { left, right, .. } = &nodes[1] {
                assert!(matches!(**left, ASTNode::IntegerLiteral(42)));
            }
        }
    }

    #[test]
    fn test_dead_branch_elimination() {
        let mut propagator = ConstantPropagator::new();
        
        let ast = ASTNode::If {
            condition: Box::new(ASTNode::BooleanLiteral(true)),
            then_branch: vec![ASTNode::IntegerLiteral(1)],
            else_branch: Some(vec![ASTNode::IntegerLiteral(2)]),
        };

        let result = propagator.optimize(&ast).unwrap();
        assert!(propagator.modified);
        
        // Should be optimized to just the then branch
        assert!(matches!(result, ASTNode::Block(nodes) if nodes.len() == 1));
    }

    #[test]
    fn test_float_operations() {
        let mut propagator = ConstantPropagator::new();
        let ast = ASTNode::BinaryOperation {
            left: Box::new(ASTNode::FloatLiteral(2.5)),
            operator: BinaryOperator::Add,
            right: Box::new(ASTNode::FloatLiteral(3.5)),
        };

        let result = propagator.optimize(&ast).unwrap();
        assert!(matches!(result, ASTNode::FloatLiteral(6.0)));
    }

    #[test]
    fn test_string_concatenation() {
        let mut propagator = ConstantPropagator::new();
        let ast = ASTNode::BinaryOperation {
            left: Box::new(ASTNode::StringLiteral("Hello, ".to_string())),
            operator: BinaryOperator::Add,
            right: Box::new(ASTNode::StringLiteral("World!".to_string())),
        };

        let result = propagator.optimize(&ast).unwrap();
        assert!(matches!(result, ASTNode::StringLiteral(s) if s == "Hello, World!"));
    }

    #[test]
    fn test_mixed_numeric_operations() {
        let mut propagator = ConstantPropagator::new();
        let ast = ASTNode::BinaryOperation {
            left: Box::new(ASTNode::IntegerLiteral(2)),
            operator: BinaryOperator::Multiply,
            right: Box::new(ASTNode::FloatLiteral(3.5)),
        };

        let result = propagator.optimize(&ast).unwrap();
        assert!(matches!(result, ASTNode::FloatLiteral(7.0)));
    }
}
