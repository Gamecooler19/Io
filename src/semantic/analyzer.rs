use crate::{
    ast::{ASTNode, ASTVisitor, BinaryOperator, Parameter},
    error::IoError,
    symbol_table::{Scope, Symbol},
    types::Type,
    Result,
};

pub struct SemanticAnalyzer {
    current_scope: Box<Scope>,
    in_loop: bool,
    in_function: bool,
    return_type: Option<Type>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            current_scope: Box::new(Scope::new()),
            in_loop: false,
            in_function: false,
            return_type: None,
        }
    }

    fn enter_scope(&mut self) {
        let new_scope = Box::new(Scope::with_parent(self.current_scope.clone()));
        self.current_scope = new_scope;
    }

    fn exit_scope(&mut self) {
        if let Some(parent) = self.current_scope.get_parent() {
            self.current_scope = parent;
        }
    }

    fn check_binary_operation(
        &mut self,
        left: &ASTNode,
        op: &BinaryOperator,
        right: &ASTNode,
    ) -> Result<Type> {
        let left_type = self.analyze(left)?;
        let right_type = self.analyze(right)?;

        match op {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => match (&left_type, &right_type) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::Float, Type::Float) => Ok(Type::Float),
                _ => Err(IoError::type_error(format!(
                    "Invalid operand types for arithmetic operation: {} and {}",
                    left_type, right_type
                ))),
            },
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                if left_type == right_type {
                    Ok(Type::Bool)
                } else {
                    Err(IoError::type_error(format!(
                        "Cannot compare values of different types: {} and {}",
                        left_type, right_type
                    )))
                }
            }
            // Add more operator type checking...
            _ => Err(IoError::type_error("Unsupported operator")),
        }
    }

    fn analyze(&mut self, node: &ASTNode) -> Result<Type> {
        match node {
            ASTNode::VariableDeclaration {
                name,
                type_annotation,
                value,
                is_mutable,
            } => {
                let value_type = self.analyze(value)?;

                if let Some(type_name) = type_annotation {
                    let declared_type = Type::from_str(type_name)?;
                    if !self.types_match(&declared_type, &value_type) {
                        return Err(IoError::type_error(format!(
                            "Type mismatch: expected {}, found {}",
                            declared_type, value_type
                        )));
                    }
                }

                self.current_scope.define(
                    name.clone(),
                    Symbol::Variable {
                        name: name.clone(),
                        type_name: value_type.to_string(),
                        mutable: *is_mutable,
                    },
                )?;

                Ok(value_type)
            }

            ASTNode::Assignment { target, value } => {
                let target_type = self.analyze(target)?;
                let value_type = self.analyze(value)?;

                if !self.types_match(&target_type, &value_type) {
                    return Err(IoError::type_error(format!(
                        "Cannot assign {} to variable of type {}",
                        value_type, target_type
                    )));
                }

                // Check if target is mutable
                if let ASTNode::Identifier(name) = &**target {
                    if let Some(Symbol::Variable { mutable, .. }) = self.current_scope.lookup(name)
                    {
                        if !mutable {
                            return Err(IoError::type_error(format!(
                                "Cannot assign to immutable variable {}",
                                name
                            )));
                        }
                    }
                }

                Ok(target_type)
            }

            ASTNode::BinaryOperation {
                left,
                operator,
                right,
            } => self.check_binary_operation(left, operator, right),

            ASTNode::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_type = self.analyze(condition)?;
                if !matches!(cond_type, Type::Bool) {
                    return Err(IoError::type_error(
                        "If condition must be a boolean expression",
                    ));
                }

                self.enter_scope();
                for stmt in then_branch {
                    self.analyze(stmt)?;
                }
                self.exit_scope();

                if let Some(else_stmts) = else_branch {
                    self.enter_scope();
                    for stmt in else_stmts {
                        self.analyze(stmt)?;
                    }
                    self.exit_scope();
                }

                Ok(Type::Unit)
            }

            ASTNode::While { condition, body } => {
                let cond_type = self.analyze(condition)?;
                if !matches!(cond_type, Type::Bool) {
                    return Err(IoError::type_error(
                        "While condition must be a boolean expression",
                    ));
                }

                let was_in_loop = self.in_loop;
                self.in_loop = true;

                self.enter_scope();
                for stmt in body {
                    self.analyze(stmt)?;
                }
                self.exit_scope();

                self.in_loop = was_in_loop;
                Ok(Type::Unit)
            }

            ASTNode::Return(value) => {
                if !self.in_function {
                    return Err(IoError::type_error("Return statement outside function"));
                }

                match (value, &self.return_type) {
                    (Some(expr), Some(expected)) => {
                        let actual = self.analyze(expr)?;
                        if !self.types_match(expected, &actual) {
                            return Err(IoError::type_error(format!(
                                "Return type mismatch: expected {}, found {}",
                                expected, actual
                            )));
                        }
                    }
                    (None, Some(_)) => {
                        return Err(IoError::type_error("Expected return value"));
                    }
                    (Some(_), None) => {
                        return Err(IoError::type_error("Unexpected return value"));
                    }
                    (None, None) => {}
                }

                Ok(Type::Unit)
            }

            ASTNode::CallExpression { callee, arguments } => {
                let callee_type = self.analyze(callee)?;

                match callee_type {
                    Type::Function {
                        params,
                        return_type,
                        ..
                    } => {
                        if arguments.len() != params.len() {
                            return Err(IoError::type_error(format!(
                                "Expected {} arguments, found {}",
                                params.len(),
                                arguments.len()
                            )));
                        }

                        for (param, arg) in params.iter().zip(arguments) {
                            let arg_type = self.analyze(arg)?;
                            if !self.types_match(param, &arg_type) {
                                return Err(IoError::type_error(format!(
                                    "Type mismatch in function call: expected {}, found {}",
                                    param, arg_type
                                )));
                            }
                        }

                        Ok(*return_type)
                    }
                    _ => Err(IoError::type_error("Called value is not a function")),
                }
            }

            // TODO: Add handling for other node types...
            _ => Ok(Type::Unit),
        }
    }

    fn types_match(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            (Type::Int, Type::Int)
            | (Type::Float, Type::Float)
            | (Type::Bool, Type::Bool)
            | (Type::String, Type::String) => true,
            (Type::Array(t1), Type::Array(t2)) => self.types_match(t1, t2),
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
                    && p1.iter().zip(p2).all(|(a, b)| self.types_match(a, b))
                    && self.types_match(r1, r2)
            }
            _ => false,
        }
    }
}

impl ASTVisitor for SemanticAnalyzer {
    type Output = Type;

    fn visit_program(&mut self, nodes: &[ASTNode]) -> Result<Self::Output> {
        for node in nodes {
            self.analyze(node)?;
        }
        Ok(Type::Unit)
    }

    fn visit_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_type: &Option<String>,
        body: &[ASTNode],
        is_async: bool,
    ) -> Result<Self::Output> {
        let was_in_function = self.in_function;
        self.in_function = true;

        let return_type = if let Some(rt) = return_type {
            Type::from_str(rt)?
        } else {
            Type::Unit
        };

        self.enter_scope();

        // Register parameters in function scope
        for param in params {
            let param_type = Type::from_str(&param.type_annotation)?;
            self.current_scope.define(
                param.name.clone(),
                Symbol::Variable {
                    name: param.name.clone(),
                    type_name: param_type.to_string(),
                    mutable: false,
                },
            )?;
        }

        let old_return_type = self.return_type.replace(return_type.clone());

        // Analyze function body
        for node in body {
            self.analyze(node)?;
        }

        self.exit_scope();
        self.in_function = was_in_function;
        self.return_type = old_return_type;

        Ok(Type::Function {
            params: params
                .iter()
                .map(|p| Type::from_str(&p.type_annotation))
                .collect::<Result<Vec<_>>>()?,
            return_type: Box::new(return_type),
            is_async,
        })
    }

    // TODO: Implement other visitor methods...
}
