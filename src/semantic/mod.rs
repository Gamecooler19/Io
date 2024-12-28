pub mod analyzer;
pub mod scope;

use crate::{ast::ASTNode, Result};

pub fn analyze(ast: &ASTNode) -> Result<()> {
    let mut analyzer = analyzer::SemanticAnalyzer::new();
    analyzer.visit_program(match ast {
        ASTNode::Program(nodes) => nodes,
        _ => return Err(crate::IoError::type_error("Expected program root")),
    })?;
    Ok(())
}

pub fn validate_control_flow(ast: &ASTNode) -> Result<()> {
    let mut validator = ControlFlowValidator::new();
    validator.validate(ast)
}

struct ControlFlowValidator {
    in_loop: bool,
    in_function: bool,
    has_return: bool,
    loop_depth: usize,
}

impl ControlFlowValidator {
    fn new() -> Self {
        Self {
            in_loop: false,
            in_function: false,
            has_return: false,
            loop_depth: 0,
        }
    }

    fn validate(&mut self, node: &ASTNode) -> Result<()> {
        match node {
            ASTNode::Function { body, .. } => {
                let was_in_function = self.in_function;
                let had_return = self.has_return;

                self.in_function = true;
                self.has_return = false;

                for stmt in body {
                    self.validate(stmt)?;
                }

                if !self.has_return {
                    return Err(IoError::type_error("Function must return a value"));
                }

                self.in_function = was_in_function;
                self.has_return = had_return;
                Ok(())
            }

            ASTNode::Return(_) => {
                if !self.in_function {
                    return Err(IoError::type_error("Return statement outside function"));
                }
                self.has_return = true;
                Ok(())
            }

            ASTNode::Break => {
                if !self.in_loop {
                    return Err(IoError::type_error("Break statement outside loop"));
                }
                Ok(())
            }

            ASTNode::Continue => {
                if !self.in_loop {
                    return Err(IoError::type_error("Continue statement outside loop"));
                }
                Ok(())
            }

            ASTNode::While { condition, body } => {
                self.validate(condition)?;

                self.loop_depth += 1;
                let was_in_loop = self.in_loop;
                self.in_loop = true;

                for stmt in body {
                    self.validate(stmt)?;
                }

                self.loop_depth -= 1;
                self.in_loop = was_in_loop;
                Ok(())
            }

            ASTNode::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.validate(condition)?;

                let mut then_returns = false;
                let mut else_returns = false;

                for stmt in then_branch {
                    self.validate(stmt)?;
                    if self.has_return {
                        then_returns = true;
                    }
                }

                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.validate(stmt)?;
                        if self.has_return {
                            else_returns = true;
                        }
                    }
                }

                self.has_return = then_returns && else_returns;
                Ok(())
            }

            _ => Ok(()),
        }
    }
}
