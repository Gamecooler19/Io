use crate::{
    ast::{ASTNode, BinaryOperator, Parameter},
    error::IoError,
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
            ("i32", Type::Int),
            ("i64", Type::Int),
            ("f32", Type::Float),
            ("f64", Type::Float),
            ("bool", Type::Bool),
            ("str", Type::String),
            ("void", Type::Unit),
        ];

        for (name, ty) in builtins {
            self.type_env.insert(name.to_string(), ty);
        }
    }

    pub fn check_node(&mut self, node: &ASTNode) -> Result<Type> {
        match node {
            ASTNode::Function {
                name,
                params,
                return_type,
                body,
                is_async,
                location: _,
            } => self.check_function(name, params, return_type, body, *is_async),
            ASTNode::BinaryOp { op, left, right } => self.check_binary_op(op, left, right),
            ASTNode::Call { name, args } => self.check_call(name, args),
            ASTNode::Identifier(name) => self.check_identifier(name),
            ASTNode::IntegerLiteral(_) => Ok(Type::Int),
            ASTNode::FloatLiteral(_) => Ok(Type::Float),
            ASTNode::BooleanLiteral(_) => Ok(Type::Bool),
            ASTNode::StringLiteral(_) => Ok(Type::String),
            ASTNode::Return { value } => self.check_return(value.as_deref()),
            ASTNode::Loop { body } => self.check_loop(body),
            ASTNode::Break => self.check_break(),
            ASTNode::Continue => self.check_continue(),
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
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                if left_type != right_type {
                    bail!("Type mismatch in comparison")
                }
                Ok(Type::Bool)
            }
            BinaryOperator::LessThan
            | BinaryOperator::LessThanEqual
            | BinaryOperator::GreaterThan
            | BinaryOperator::GreaterThanEqual => {
                if !matches!(left_type, Type::Int | Type::Float) || left_type != right_type {
                    bail!("Invalid types for comparison")
                }
                Ok(Type::Bool)
            }
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => {
                if !self.types_match(&left_type, &right_type) {
                    return Err(IoError::type_error(format!(
                        "Type mismatch in binary operation: {:?} vs {:?}",
                        left_type, right_type
                    )));
                }
                if !matches!(left_type, Type::Int | Type::Float) {
                    return Err(IoError::type_error(
                        "Arithmetic operations only support numeric types",
                    ));
                }
                Ok(left_type)
            }
            BinaryOperator::Equals | BinaryOperator::NotEquals => {
                if !self.types_match(&left_type, &right_type) {
                    return Err(IoError::type_error("Can't compare different types"));
                }
                Ok(Type::Bool)
            }
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => {
                if !self.types_match(&left_type, &right_type)
                    || !matches!(left_type, Type::Int | Type::Float)
                {
                    return Err(IoError::type_error("Invalid types for comparison"));
                }
                Ok(Type::Bool)
            }
            BinaryOperator::And | BinaryOperator::Or => {
                if !matches!(left_type, Type::Bool) || !matches!(right_type, Type::Bool) {
                    return Err(IoError::type_error(
                        "Logical operators require boolean operands",
                    ));
                }
                Ok(Type::Bool)
            }
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

    fn check_return(&mut self, value: Option<&ASTNode>) -> Result<Type> {
        match value {
            Some(expr) => self.check_node(expr),
            None => Ok(Type::Void),
        }
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
