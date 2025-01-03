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
            BinaryOperator::LessThan | BinaryOperator::LessThanOrEqual |
            BinaryOperator::GreaterThan | BinaryOperator::GreaterThanOrEqual => {
                match (&left_type, &right_type) {
                    (Type::Int, Type::Int) | (Type::Float, Type::Float) => Ok(Type::Bool),
                    _ => Err(IoError::type_error(format!(
                        "Cannot compare values of types {} and {}",
                        left_type, right_type
                    )))
                }
            },
            BinaryOperator::And | BinaryOperator::Or => {
                if left_type == Type::Bool && right_type == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(IoError::type_error("Logical operators require boolean operands"))
                }
            },
            BinaryOperator::BitwiseAnd | BinaryOperator::BitwiseOr | 
            BinaryOperator::BitwiseXor => {
                match (&left_type, &right_type) {
                    (Type::Int, Type::Int) => Ok(Type::Int),
                    _ => Err(IoError::type_error("Bitwise operators require integer operands"))
                }
            },
            BinaryOperator::LeftShift | BinaryOperator::RightShift => {
                match (&left_type, &right_type) {
                    (Type::Int, Type::Int) => Ok(Type::Int),
                    _ => Err(IoError::type_error("Shift operators require integer operands"))
                }
            },
            BinaryOperator::Modulo => match (&left_type, &right_type) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::Float, Type::Float) => Ok(Type::Float),
                _ => Err(IoError::type_error("Modulo operator requires numeric operands"))
            },
            BinaryOperator::Concat => match (&left_type, &right_type) {
                (Type::String, Type::String) => Ok(Type::String),
                (Type::Array(t1), Type::Array(t2)) if t1 == t2 => Ok(Type::Array(t1.clone())),
                _ => Err(IoError::type_error("Concatenation requires matching string or array types"))
            },
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

            ASTNode::Break => {
                if !self.in_loop {
                    Err(IoError::type_error("Break statement outside loop"))
                } else {
                    Ok(Type::Unit)
                }
            },
            ASTNode::Continue => {
                if !self.in_loop {
                    Err(IoError::type_error("Continue statement outside loop"))
                } else {
                    Ok(Type::Unit)
                }
            },
            ASTNode::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    Ok(Type::Array(Box::new(Type::Unknown)))
                } else {
                    let first_type = self.analyze(&elements[0])?;
                    for elem in &elements[1..] {
                        let elem_type = self.analyze(elem)?;
                        if !self.types_match(&first_type, &elem_type) {
                            return Err(IoError::type_error("Array elements must have same type"));
                        }
                    }
                    Ok(Type::Array(Box::new(first_type)))
                }
            },
            ASTNode::IndexAccess { array, index } => {
                let array_type = self.analyze(array)?;
                let index_type = self.analyze(index)?;
                
                match (array_type, index_type) {
                    (Type::Array(elem_type), Type::Int) => Ok(*elem_type),
                    (Type::String, Type::Int) => Ok(Type::String),
                    _ => Err(IoError::type_error("Invalid array access"))
                }
            },
            ASTNode::StructLiteral { name, fields } => {
                if let Some(struct_type) = self.current_scope.lookup_type(name) {
                    match struct_type {
                        Type::Struct { fields: struct_fields } => {
                            for (field_name, field_value) in fields {
                                if let Some(expected_type) = struct_fields.get(field_name) {
                                    let actual_type = self.analyze(field_value)?;
                                    if !self.types_match(expected_type, &actual_type) {
                                        return Err(IoError::type_error(format!(
                                            "Type mismatch in field {}: expected {}, found {}",
                                            field_name, expected_type, actual_type
                                        )));
                                    }
                                } else {
                                    return Err(IoError::type_error(format!(
                                        "Unknown field {} in struct {}", field_name, name
                                    )));
                                }
                            }
                            Ok(struct_type)
                        },
                        _ => Err(IoError::type_error(format!("{} is not a struct type", name)))
                    }
                } else {
                    Err(IoError::type_error(format!("Undefined struct type {}", name)))
                }
            },
            ASTNode::Match { expr, arms } => {
                let expr_type = self.analyze(expr)?;
                let mut result_type = None;

                for (pattern, body) in arms {
                    self.enter_scope();
                    self.check_pattern(pattern, &expr_type)?;
                    let arm_type = self.analyze(body)?;
                    
                    if let Some(prev_type) = &result_type {
                        if !self.types_match(prev_type, &arm_type) {
                            return Err(IoError::type_error("All match arms must return same type"));
                        }
                    } else {
                        result_type = Some(arm_type);
                    }
                    self.exit_scope();
                }

                Ok(result_type.unwrap_or(Type::Unit))
            }
            // Add handling for other node types...
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

    fn visit_variable_declaration(
        &mut self,
        name: &str,
        type_annotation: &Option<String>,
        initializer: &Option<ASTNode>,
    ) -> Result<Self::Output> {
        let var_type = if let Some(type_name) = type_annotation {
            Type::from_str(type_name)?
        } else if let Some(init) = initializer {
            self.analyze(init)?
        } else {
            return Err(IoError::type_error("Cannot infer type without initializer"));
        };

        self.current_scope.define(
            name.to_string(),
            Symbol::Variable {
                name: name.to_string(),
                type_name: var_type.to_string(),
                mutable: true,
            },
        )?;

        Ok(var_type)
    }

    fn visit_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &[ASTNode],
        else_branch: &Option<Vec<ASTNode>>,
    ) -> Result<Self::Output> {
        let cond_type = self.analyze(condition)?;
        if !matches!(cond_type, Type::Bool) {
            return Err(IoError::type_error("If condition must be boolean"));
        }

        self.enter_scope();
        let then_type = self.analyze_block(then_branch)?;
        self.exit_scope();

        if let Some(else_stmts) = else_branch {
            self.enter_scope();
            let else_type = self.analyze_block(else_stmts)?;
            self.exit_scope();

            if !self.types_match(&then_type, &else_type) {
                return Err(IoError::type_error("If branches must have same type"));
            }
        }

        Ok(Type::Unit)
    }

    fn visit_match(
        &mut self,
        expr: &ASTNode,
        arms: &[(Pattern, ASTNode)],
    ) -> Result<Self::Output> {
        let expr_type = self.analyze(expr)?;
        let mut result_type = None;

        for (pattern, body) in arms {
            self.enter_scope();
            self.check_pattern(pattern, &expr_type)?;
            let arm_type = self.analyze(body)?;
            
            if let Some(prev_type) = &result_type {
                if !self.types_match(prev_type, &arm_type) {
                    return Err(IoError::type_error("All match arms must return same type"));
                }
            } else {
                result_type = Some(arm_type);
            }
            self.exit_scope();
        }

        Ok(result_type.unwrap_or(Type::Unit))
    }

    fn visit_binary_op(
        &mut self,
        left: &ASTNode,
        op: &BinaryOperator,
        right: &ASTNode,
    ) -> Result<Self::Output> {
        self.check_binary_operation(left, op, right)
    }

    fn visit_unary_op(&mut self, op: &str, expr: &ASTNode) -> Result<Self::Output> {
        let expr_type = self.analyze(expr)?;
        match op {
            "-" => match expr_type {
                Type::Int | Type::Float => Ok(expr_type),
                _ => Err(IoError::type_error("Unary minus requires numeric operand"))
            },
            "!" => match expr_type {
                Type::Bool => Ok(Type::Bool),
                _ => Err(IoError::type_error("Logical not requires boolean operand"))
            },
            "~" => match expr_type {
                Type::Int => Ok(Type::Int),
                _ => Err(IoError::type_error("Bitwise not requires integer operand"))
            },
            _ => Err(IoError::type_error("Unknown unary operator"))
        }
    }

    fn visit_for(&mut self, init: &ASTNode, cond: &ASTNode, step: &ASTNode, body: &[ASTNode]) -> Result<Self::Output> {
        self.enter_scope();
        
        // Check initialization
        self.analyze(init)?;
        
        // Verify condition is boolean
        let cond_type = self.analyze(cond)?;
        if !matches!(cond_type, Type::Bool) {
            return Err(IoError::type_error("For loop condition must be boolean"));
        }
        
        // Analyze step expression
        self.analyze(step)?;
        
        // Handle loop body
        let was_in_loop = self.in_loop;
        self.in_loop = true;
        
        for node in body {
            self.analyze(node)?;
        }
        
        self.in_loop = was_in_loop;
        self.exit_scope();
        
        Ok(Type::Unit)
    }

    fn visit_struct_declaration(&mut self, name: &str, fields: &[(String, String)]) -> Result<Self::Output> {
        let field_types = fields.iter()
            .map(|(name, type_name)| {
                let field_type = Type::from_str(type_name)?;
                Ok((name.clone(), field_type))
            })
            .collect::<Result<HashMap<String, Type>>>()?;

        let struct_type = Type::Struct {
            name: name.to_string(),
            fields: field_types,
        };

        self.current_scope.define_type(name.to_string(), struct_type.clone())?;
        Ok(struct_type)
    }

    fn visit_enum_declaration(&mut self, name: &str, variants: &[(String, Vec<Type>)]) -> Result<Self::Output> {
        let enum_type = Type::Enum {
            name: name.to_string(),
            variants: variants.iter()
                .map(|(variant, types)| (variant.clone(), types.clone()))
                .collect(),
        };

        self.current_scope.define_type(name.to_string(), enum_type.clone())?;
        Ok(enum_type)
    }

    fn visit_try_catch(&mut self, try_block: &[ASTNode], catch_blocks: &[(String, String, Vec<ASTNode>)], finally: &Option<Vec<ASTNode>>) -> Result<Self::Output> {
        // Analyze try block
        self.enter_scope();
        for node in try_block {
            self.analyze(node)?;
        }
        self.exit_scope();

        // Analyze catch blocks
        for (exception_type, var_name, block) in catch_blocks {
            self.enter_scope();
            
            // Register exception variable
            let exc_type = Type::from_str(exception_type)?;
            self.current_scope.define(
                var_name.clone(),
                Symbol::Variable {
                    name: var_name.clone(),
                    type_name: exc_type.to_string(),
                    mutable: false,
                },
            )?;

            // Analyze catch block
            for node in block {
                self.analyze(node)?;
            }
            self.exit_scope();
        }

        // Analyze finally block
        if let Some(finally_block) = finally {
            self.enter_scope();
            for node in finally_block {
                self.analyze(node)?;
            }
            self.exit_scope();
        }

        Ok(Type::Unit)
    }

    fn visit_async_block(&mut self, block: &[ASTNode]) -> Result<Self::Output> {
        self.enter_scope();
        
        let mut block_type = Type::Unit;
        for node in block {
            block_type = self.analyze(node)?;
        }
        
        self.exit_scope();
        
        Ok(Type::Future(Box::new(block_type)))
    }

    fn visit_await_expr(&mut self, expr: &ASTNode) -> Result<Self::Output> {
        let expr_type = self.analyze(expr)?;
        match expr_type {
            Type::Future(inner_type) => Ok(*inner_type),
            _ => Err(IoError::type_error("Can only await Future types")),
        }
    }
}

impl SemanticAnalyzer {
    // Add new analysis methods for additional node types
    fn analyze_pattern(&mut self, pattern: &Pattern, value_type: &Type) -> Result<()> {
        match pattern {
            Pattern::Literal(lit) => {
                let lit_type = self.analyze(lit)?;
                if !self.types_match(&lit_type, value_type) {
                    return Err(IoError::type_error(format!(
                        "Pattern type mismatch: expected {}, found {}",
                        value_type, lit_type
                    )));
                }
            }
            Pattern::Variable(name) => {
                self.current_scope.define(
                    name.clone(),
                    Symbol::Variable {
                        name: name.clone(),
                        type_name: value_type.to_string(),
                        mutable: false,
                    },
                )?;
            }
            Pattern::Constructor { name, patterns } => {
                if let Some(Type::Enum { variants, .. }) = self.current_scope.lookup_type(name) {
                    if let Some(variant_types) = variants.get(name) {
                        if patterns.len() != variant_types.len() {
                            return Err(IoError::type_error(format!(
                                "Wrong number of fields for enum variant {}", name
                            )));
                        }
                        for (pattern, expected_type) in patterns.iter().zip(variant_types) {
                            self.analyze_pattern(pattern, expected_type)?;
                        }
                    }
                }
            }
            Pattern::Tuple(patterns) => {
                if let Type::Tuple(types) = value_type {
                    if patterns.len() != types.len() {
                        return Err(IoError::type_error("Tuple pattern length mismatch"));
                    }
                    for (pattern, ty) in patterns.iter().zip(types) {
                        self.analyze_pattern(pattern, ty)?;
                    }
                }
            }
            Pattern::Rest => {} // Rest pattern matches anything
        }
        Ok(())
    }

    fn analyze_block(&mut self, nodes: &[ASTNode]) -> Result<Type> {
        let mut block_type = Type::Unit;
        for node in nodes {
            block_type = self.analyze(node)?;
        }
        Ok(block_type)
    }
}
