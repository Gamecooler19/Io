use crate::{
    ast::{Pattern, ASTNode},
    types::Type,
    error::IoError,
    symbol_table::{Scope, Symbol},
    Result,
};

#[derive(Debug)]
pub struct PatternMatcher {
    scope: Box<Scope>,
}

impl PatternMatcher {
    pub fn new(parent_scope: Box<Scope>) -> Self {
        Self {
            scope: Box::new(Scope::with_parent(parent_scope)),
        }
    }

    pub fn match_pattern(&mut self, pattern: &Pattern, value: &ASTNode, value_type: &Type) -> Result<()> {
        match pattern {
            Pattern::Literal(lit) => self.match_literal(lit, value),
            Pattern::Variable(name) => self.bind_variable(name, value, value_type),
            Pattern::Wildcard => Ok(()), // Always matches
            Pattern::Constructor { name, fields } => self.match_constructor(name, fields, value),
        }
    }

    fn match_literal(&self, lit: &ASTNode, value: &ASTNode) -> Result<()> {
        match (lit, value) {
            (ASTNode::IntegerLiteral(l), ASTNode::IntegerLiteral(r)) if l == r => Ok(()),
            (ASTNode::StringLiteral(l), ASTNode::StringLiteral(r)) if l == r => Ok(()),
            (ASTNode::BooleanLiteral(l), ASTNode::BooleanLiteral(r)) if l == r => Ok(()),
            _ => Err(IoError::runtime_error("Pattern match failed")),
        }
    }

    fn bind_variable(&mut self, name: &str, value: &ASTNode, value_type: &Type) -> Result<()> {
        self.scope.define(
            name.to_string(),
            Symbol::Variable {
                name: name.to_string(),
                type_name: value_type.to_string(),
                mutable: false,
            },
        )
    }

    fn match_constructor(&mut self, name: &str, fields: &[Pattern], value: &ASTNode) -> Result<()> {
        match value {
            ASTNode::CallExpression { callee, arguments } => {
                if let ASTNode::Identifier(callee_name) = &**callee {
                    // Verify constructor exists
                    let constructor_type = self.scope.lookup_type(callee_name)
                        .ok_or_else(|| IoError::runtime_error(
                            format!("Unknown constructor: {}", callee_name)
                        ))?;

                    // Verify arity matches
                    if arguments.len() != fields.len() {
                        return Err(IoError::runtime_error(format!(
                            "Constructor {} expects {} arguments, but got {}",
                            name,
                            fields.len(),
                            arguments.len()
                        )));
                    }

                    // Get field types from constructor definition
                    let field_types = self.get_constructor_field_types(constructor_type)?;

                    // Match each field with corresponding argument
                    for ((field, arg), field_type) in fields.iter().zip(arguments).zip(field_types) {
                        // Infer argument type
                        let arg_type = self.infer_type(arg)?;
                        
                        // Verify type compatibility
                        if !self.types_compatible(&field_type, &arg_type) {
                            return Err(IoError::type_error(format!(
                                "Type mismatch in constructor {}: expected {}, found {}",
                                name, field_type, arg_type
                            )));
                        }

                        // Match the pattern
                        self.match_pattern(field, arg, &field_type)?;
                    }

                    Ok(())
                } else {
                    Err(IoError::runtime_error("Expected constructor identifier"))
                }
            }
            _ => Err(IoError::runtime_error("Expected constructor application")),
        }
    }

    fn get_constructor_field_types(&self, constructor_type: &Type) -> Result<Vec<Type>> {
        match constructor_type {
            Type::Constructor { fields, .. } => Ok(fields.clone()),
            Type::Enum { variants, .. } => {
                // Handle enum constructors
                Ok(variants.iter()
                    .flat_map(|v| v.fields.clone())
                    .collect())
            }
            _ => Err(IoError::type_error(format!(
                "Expected constructor type, found {:?}",
                constructor_type
            ))),
        }
    }

    fn infer_type(&self, node: &ASTNode) -> Result<Type> {
        match node {
            ASTNode::IntegerLiteral(_) => Ok(Type::Integer),
            ASTNode::FloatLiteral(_) => Ok(Type::Float),
            ASTNode::StringLiteral(_) => Ok(Type::String),
            ASTNode::BooleanLiteral(_) => Ok(Type::Boolean),
            ASTNode::ArrayLiteral(elements) => {
                if elements.is_empty() {
                    Ok(Type::Array(Box::new(Type::Unknown)))
                } else {
                    let element_type = self.infer_type(&elements[0])?;
                    // Verify all elements have same type
                    for element in &elements[1..] {
                        let t = self.infer_type(element)?;
                        if !self.types_compatible(&element_type, &t) {
                            return Err(IoError::type_error(
                                "Inconsistent array element types"
                            ));
                        }
                    }
                    Ok(Type::Array(Box::new(element_type)))
                }
            }
            ASTNode::Identifier(name) => {
                self.scope.lookup_type(name)
                    .ok_or_else(|| IoError::runtime_error(
                        format!("Cannot infer type of undefined variable: {}", name)
                    ))
            }
            ASTNode::CallExpression { callee, arguments } => {
                if let ASTNode::Identifier(name) = &**callee {
                    let fn_type = self.scope.lookup_type(name)
                        .ok_or_else(|| IoError::runtime_error(
                            format!("Unknown function: {}", name)
                        ))?;
                    
                    match fn_type {
                        Type::Function { return_type, .. } => Ok(*return_type),
                        _ => Err(IoError::type_error("Expected function type")),
                    }
                } else {
                    Err(IoError::runtime_error("Expected function identifier"))
                }
            }
            _ => Err(IoError::runtime_error(
                format!("Cannot infer type of {:?}", node)
            )),
        }
    }

    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            // Basic type equality
            (Type::Integer, Type::Integer) |
            (Type::Float, Type::Float) |
            (Type::String, Type::String) |
            (Type::Boolean, Type::Boolean) => true,

            // Array type compatibility
            (Type::Array(t1), Type::Array(t2)) => 
                self.types_compatible(t1, t2),

            // Function type compatibility
            (Type::Function { params: p1, return_type: r1 },
             Type::Function { params: p2, return_type: r2 }) => {
                p1.len() == p2.len() &&
                p1.iter().zip(p2).all(|(t1, t2)| self.types_compatible(t1, t2)) &&
                self.types_compatible(r1, r2)
            }

            // Constructor type compatibility
            (Type::Constructor { name: n1, fields: f1 },
             Type::Constructor { name: n2, fields: f2 }) => {
                n1 == n2 && f1.len() == f2.len() &&
                f1.iter().zip(f2).all(|(t1, t2)| self.types_compatible(t1, t2))
            }

            // Unknown type is compatible with anything
            (_, Type::Unknown) | (Type::Unknown, _) => true,

            // Everything else is incompatible
            _ => false,
        }
    }
}
