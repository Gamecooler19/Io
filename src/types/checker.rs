use crate::{
    ast::{ASTNode, BinaryOperator, Parameter},
    error::{self, IoError},
    types::Type,
    Result,
};
use std::collections::HashMap;

pub struct TypeChecker {
    type_env: HashMap<String, Type>,
    current_function_return_type: Option<Type>,
    in_loop: bool,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            type_env: HashMap::new(),
            current_function_return_type: None,
            in_loop: false,
        };
        checker.init_builtin_types();
        checker
    }

    fn init_builtin_types(&mut self) {
        let builtins = vec![
            ("i32", Type::I32),
            ("i64", Type::I64),
            ("f32", Type::F32),
            ("f64", Type::F64),
            ("bool", Type::Bool),
            ("str", Type::String),
            ("void", Type::Void),
        ];

        for (name, ty) in builtins {
            self.type_env.insert(name.to_string(), ty);
        }
    }

    fn check_node(&mut self, node: &ASTNode) -> Result<Type> {
        match node {
            ASTNode::IntLiteral { .. } => Ok(Type::Int),
            ASTNode::FloatLiteral { .. } => Ok(Type::Float),
            ASTNode::StringLiteral { .. } => Ok(Type::String),
            ASTNode::BoolLiteral { .. } => Ok(Type::Bool),
            ASTNode::Identifier { name, .. } => self.check_identifier(name),
            ASTNode::BinaryOp {
                op, left, right, ..
            } => self.check_binary_op(op, left, right),
            ASTNode::Call { name, args, .. } => self.check_call(name, args),
            ASTNode::Function {
                name,
                params,
                return_type,
                body,
                is_async,
                ..
            } => self.check_function(name, params, return_type, body, *is_async),
            ASTNode::Return { value, .. } => self.check_return(value),
            ASTNode::Block { statements, .. } => self.check_block(statements),
            _ => Err(IoError::type_error("Unsupported node type")),
        }
    }

    fn check_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_type: &Option<String>,
        body: &[ASTNode],
        is_async: bool,
    ) -> Result<Type> {
        let param_types = params
            .iter()
            .map(|p| self.resolve_type(&p.type_annotation))
            .collect::<Result<Vec<_>>>()?;

        let ret_type = match return_type {
            Some(t) => self.resolve_type(t)?,
            None => Type::Unit,
        };

        // Store return type for checking returns in function body
        self.current_function_return_type = Some(ret_type.clone());

        let fn_type = Type::Function {
            params: param_types.clone(),
            return_type: Box::new(ret_type.clone()),
            is_async,
        };

        // Add function to environment
        self.type_env.insert(name.to_string(), fn_type.clone());

        // Add parameters to environment
        let prev_env = self.type_env.clone();
        for (param, param_type) in params.iter().zip(param_types.iter()) {
            self.type_env.insert(param.name.clone(), param_type.clone());
        }

        // Check body
        let mut block_type = Type::Unit;
        for node in body {
            block_type = self.check_node(node)?;
        }

        // Verify return type
        if !self.types_match(&block_type, &ret_type) {
            return Err(IoError::type_error(format!(
                "Function return type mismatch. Expected {:?}, found {:?}",
                ret_type, block_type
            )));
        }

        // Restore environment
        self.type_env = prev_env;
        self.current_function_return_type = None;

        Ok(fn_type)
    }

    fn check_binary_op(
        &mut self,
        op: &BinaryOperator,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<Type> {
        let left_type = self.check_node(left)?;
        let right_type = self.check_node(right)?;

        match op {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => match (&left_type, &right_type) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::Float, Type::Float) => Ok(Type::Float),
                _ => Err(error::IoError::type_error("Invalid operand types")),
            },
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                if left_type != right_type {
                    return Err(IoError::type_error("Type mismatch in comparison"));
                }
                Ok(Type::Bool)
            }
            BinaryOperator::LessThan
            | BinaryOperator::LessThanEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanEqual => match (&left_type, &right_type) {
                (Type::Int | Type::Float, Type::Int | Type::Float) => Ok(Type::Bool),
                _ => Err(error::IoError::type_error("Invalid comparison types")),
            },
            BinaryOperator::And | BinaryOperator::Or => match (&left_type, &right_type) {
                (Type::Bool, Type::Bool) => Ok(Type::Bool),
                _ => Err(IoError::type_error(
                    "Logical operators require boolean operands",
                )),
            },
        }
    }

    fn check_call(&mut self, name: &str, args: &[ASTNode]) -> Result<Type> {
        let fn_type = self.resolve_type(name)?;

        match fn_type {
            Type::Function {
                params,
                return_type,
                ..
            } => {
                if args.len() != params.len() {
                    return Err(IoError::type_error(format!(
                        "Function {} expects {} arguments but got {}",
                        name,
                        params.len(),
                        args.len()
                    )));
                }

                for (arg, param_type) in args.iter().zip(params.iter()) {
                    let arg_type = self.check_node(arg)?;
                    if !self.types_match(&arg_type, param_type) {
                        return Err(IoError::type_error(format!(
                            "Argument type mismatch: expected {:?}, got {:?}",
                            param_type, arg_type
                        )));
                    }
                }

                Ok(*return_type)
            }
            _ => Err(IoError::type_error(format!("{} is not a function", name))),
        }
    }

    fn check_return(&self, value: Option<&ASTNode>) -> Result<Type> {
        let return_type = self
            .current_function_return_type
            .as_ref()
            .ok_or_else(|| IoError::type_error("Return statement outside of function"))?;

        match value {
            Some(expr) => {
                let expr_type = self.check_node(expr)?;
                if !self.types_match(&expr_type, return_type) {
                    return Err(IoError::type_error(format!(
                        "Return type mismatch: expected {:?}, got {:?}",
                        return_type, expr_type
                    )));
                }
            }
            None => {
                if !matches!(return_type, Type::Unit) {
                    return Err(IoError::type_error(format!(
                        "Function must return {:?}",
                        return_type
                    )));
                }
            }
        }

        Ok(Type::Unit)
    }

    fn check_loop(&mut self, body: &[ASTNode]) -> Result<Type> {
        let was_in_loop = self.in_loop;
        self.in_loop = true;

        for node in body {
            self.check_node(node)?;
        }

        self.in_loop = was_in_loop;
        Ok(Type::Unit)
    }

    fn check_break(&self) -> Result<Type> {
        if !self.in_loop {
            return Err(IoError::type_error("Break statement outside of loop"));
        }
        Ok(Type::Unit)
    }

    fn check_continue(&self) -> Result<Type> {
        if !self.in_loop {
            return Err(IoError::type_error("Continue statement outside of loop"));
        }
        Ok(Type::Unit)
    }

    fn check_identifier(&self, name: &str) -> Result<Type> {
        self.resolve_type(name)
    }

    fn resolve_type(&self, name: &str) -> Result<Type> {
        self.type_env
            .get(name)
            .cloned()
            .ok_or_else(|| IoError::type_error(format!("Unknown type or identifier: {}", name)))
    }

    fn types_match(&self, actual: &Type, expected: &Type) -> bool {
        match (actual, expected) {
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::String, Type::String)
            | (Type::Unit, Type::Unit) => true,

            (
                Type::Function {
                    params: p1,
                    return_type: r1,
                    ..
                },
                Type::Function {
                    params: p2,
                    return_type: r2,
                    ..
                },
            ) => {
                p1.len() == p2.len()
                    && p1
                        .iter()
                        .zip(p2.iter())
                        .all(|(a, b)| self.types_match(a, b))
                    && self.types_match(r1, r2)
            }

            _ => false,
        }
    }

    fn check_string_operation(
        &mut self,
        op: &BinaryOperator,
        left: &ASTNode,
        right: &ASTNode,
    ) -> Result<Type> {
        let left_type = self.check_node(left)?;
        let right_type = self.check_node(right)?;

        match (left_type, right_type) {
            (Type::String, Type::String) => {
                match op {
                    BinaryOperator::Add => Ok(Type::String), // String concatenation
                    BinaryOperator::Equal | BinaryOperator::NotEqual => Ok(Type::Bool),
                    _ => Err(IoError::type_error("Invalid string operation")),
                }
            }
            _ => Err(IoError::type_error("Operation requires string operands")),
        }
    }

    fn check_block(&mut self, statements: &[ASTNode]) -> Result<Type> {
        let mut block_type = Type::Void;
        for stmt in statements {
            block_type = self.check_node(stmt)?;
        }
        Ok(block_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_types() {
        let checker = TypeChecker::new();
        assert!(checker.types_match(&Type::Int, &Type::Int));
        assert!(checker.types_match(&Type::Float, &Type::Float));
        assert!(!checker.types_match(&Type::Int, &Type::Float));
    }

    #[test]
    fn test_function_types() {
        let mut checker = TypeChecker::new();
        let fn_type = Type::Function {
            params: vec![Type::Int, Type::Bool],
            return_type: Box::new(Type::Float),
            is_async: false,
        };

        checker
            .type_env
            .insert("test_fn".to_string(), fn_type.clone());

        let resolved = checker.resolve_type("test_fn").unwrap();
        assert!(checker.types_match(&resolved, &fn_type));
    }
}
