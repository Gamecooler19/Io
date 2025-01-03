use crate::{
    ast::{ASTNode, ASTVisitor},
    error::IoError,
    parser::Parser,
    Result,
};
use std::collections::HashMap;

#[derive(Debug)]
pub struct MacroDefinition {
    name: String,
    params: Vec<String>,
    body: Vec<ASTNode>,
}

pub struct MacroExpander {
    definitions: HashMap<String, MacroDefinition>,
    expansion_depth: usize,
    max_depth: usize,
}

impl MacroExpander {
    pub fn new(max_depth: usize) -> Self {
        Self {
            definitions: HashMap::new(),
            expansion_depth: 0,
            max_depth,
        }
    }

    pub fn register_macro(&mut self, def: MacroDefinition) -> Result<()> {
        if self.definitions.contains_key(&def.name) {
            return Err(IoError::validation_error(format!(
                "Macro '{}' is already defined",
                def.name
            )));
        }
        self.definitions.insert(def.name.clone(), def);
        Ok(())
    }

    pub fn expand(&mut self, node: &ASTNode) -> Result<ASTNode> {
        if self.expansion_depth >= self.max_depth {
            return Err(IoError::runtime_error(
                "Maximum macro expansion depth exceeded",
            ));
        }

        match node {
            ASTNode::CallExpression { callee, arguments } => {
                if let ASTNode::Identifier(name) = &**callee {
                    if let Some(macro_def) = self.definitions.get(name) {
                        self.expansion_depth += 1;
                        let expanded = self.expand_macro(macro_def, arguments)?;
                        self.expansion_depth -= 1;
                        return Ok(expanded);
                    }
                }
                self.expand_children(node)
            }
            _ => self.expand_children(node),
        }
    }

    fn expand_macro(&mut self, def: &MacroDefinition, args: &[ASTNode]) -> Result<ASTNode> {
        if args.len() != def.params.len() {
            return Err(IoError::validation_error(format!(
                "Macro '{}' expects {} arguments but got {}",
                def.name,
                def.params.len(),
                args.len()
            )));
        }

        let mut replacements = HashMap::new();
        for (param, arg) in def.params.iter().zip(args.iter()) {
            replacements.insert(param.clone(), arg.clone());
        }

        let mut expander = MacroReplacer { replacements };
        let mut expanded_body = Vec::new();

        for node in &def.body {
            expanded_body.push(expander.visit_node(node)?);
        }

        Ok(ASTNode::Block(expanded_body))
    }

    fn expand_children(&mut self, node: &ASTNode) -> Result<ASTNode> {
        match node {
            ASTNode::Block(nodes) => {
                let mut expanded = Vec::new();
                for node in nodes {
                    expanded.push(self.expand(node)?);
                }
                Ok(ASTNode::Block(expanded))
            }
            ASTNode::Function { name, params, return_type, body, is_async } => {
                let expanded_body = body.iter()
                    .map(|node| self.expand(node))
                    .collect::<Result<Vec<_>>>()?;
                Ok(ASTNode::Function {
                    name: name.clone(),
                    params: params.clone(),
                    return_type: return_type.clone(),
                    body: expanded_body,
                    is_async: *is_async,
                })
            }
            ASTNode::If { condition, then_branch, else_branch } => {
                Ok(ASTNode::If {
                    condition: Box::new(self.expand(condition)?),
                    then_branch: then_branch.iter()
                        .map(|node| self.expand(node))
                        .collect::<Result<Vec<_>>>()?,
                    else_branch: if let Some(else_nodes) = else_branch {
                        Some(else_nodes.iter()
                            .map(|node| self.expand(node))
                            .collect::<Result<Vec<_>>>()?)
                    } else {
                        None
                    },
                })
            }
            ASTNode::While { condition, body } => {
                Ok(ASTNode::While {
                    condition: Box::new(self.expand(condition)?),
                    body: body.iter()
                        .map(|node| self.expand(node))
                        .collect::<Result<Vec<_>>>()?,
                })
            }
            ASTNode::For { init, condition, update, body } => {
                Ok(ASTNode::For {
                    init: Box::new(self.expand(init)?),
                    condition: Box::new(self.expand(condition)?),
                    update: Box::new(self.expand(update)?),
                    body: body.iter()
                        .map(|node| self.expand(node))
                        .collect::<Result<Vec<_>>>()?,
                })
            }
            ASTNode::BinaryOp { left, op, right } => {
                Ok(ASTNode::BinaryOp {
                    left: Box::new(self.expand(left)?),
                    op: op.clone(),
                    right: Box::new(self.expand(right)?),
                })
            }
            // Nodes that don't need expansion
            ASTNode::IntegerLiteral(_) |
            ASTNode::FloatLiteral(_) |
            ASTNode::StringLiteral(_) |
            ASTNode::BooleanLiteral(_) |
            ASTNode::Identifier(_) => Ok(node.clone()),
        }
    }
}

struct MacroReplacer {
    replacements: HashMap<String, ASTNode>,
}

impl ASTVisitor for MacroReplacer {
    type Output = ASTNode;

    fn visit_identifier(&mut self, name: &str) -> Result<Self::Output> {
        Ok(self
            .replacements
            .get(name)
            .cloned()
            .unwrap_or_else(|| ASTNode::Identifier(name.to_string())))
    }

    fn visit_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        return_type: &Option<String>,
        body: &[ASTNode],
        is_async: bool,
    ) -> Result<Self::Output> {
        let visited_body = body.iter()
            .map(|node| self.visit_node(node))
            .collect::<Result<Vec<_>>>()?;
        
        Ok(ASTNode::Function {
            name: name.to_string(),
            params: params.clone(),
            return_type: return_type.clone(),
            body: visited_body,
            is_async,
        })
    }

    fn visit_binary_op(
        &mut self,
        left: &ASTNode,
        op: &BinaryOperator,
        right: &ASTNode,
    ) -> Result<Self::Output> {
        Ok(ASTNode::BinaryOp {
            left: Box::new(self.visit_node(left)?),
            op: op.clone(),
            right: Box::new(self.visit_node(right)?),
        })
    }

    fn visit_if(
        &mut self,
        condition: &ASTNode,
        then_branch: &[ASTNode],
        else_branch: &Option<Vec<ASTNode>>,
    ) -> Result<Self::Output> {
        Ok(ASTNode::If {
            condition: Box::new(self.visit_node(condition)?),
            then_branch: then_branch.iter()
                .map(|node| self.visit_node(node))
                .collect::<Result<Vec<_>>>()?,
            else_branch: if let Some(else_nodes) = else_branch {
                Some(else_nodes.iter()
                    .map(|node| self.visit_node(node))
                    .collect::<Result<Vec<_>>>()?)
            } else {
                None
            },
        })
    }

    fn visit_while(
        &mut self,
        condition: &ASTNode,
        body: &[ASTNode],
    ) -> Result<Self::Output> {
        Ok(ASTNode::While {
            condition: Box::new(self.visit_node(condition)?),
            body: body.iter()
                .map(|node| self.visit_node(node))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    fn visit_for(
        &mut self,
        init: &ASTNode,
        condition: &ASTNode,
        update: &ASTNode,
        body: &[ASTNode],
    ) -> Result<Self::Output> {
        Ok(ASTNode::For {
            init: Box::new(self.visit_node(init)?),
            condition: Box::new(self.visit_node(condition)?),
            update: Box::new(self.visit_node(update)?),
            body: body.iter()
                .map(|node| self.visit_node(node))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    fn visit_call(
        &mut self,
        callee: &ASTNode,
        args: &[ASTNode],
    ) -> Result<Self::Output> {
        Ok(ASTNode::CallExpression {
            callee: Box::new(self.visit_node(callee)?),
            arguments: args.iter()
                .map(|arg| self.visit_node(arg))
                .collect::<Result<Vec<_>>>()?,
        })
    }

    fn visit_variable_declaration(
        &mut self,
        name: &str,
        initializer: &Option<ASTNode>,
        type_annotation: &Option<String>,
    ) -> Result<Self::Output> {
        Ok(ASTNode::VariableDeclaration {
            name: name.to_string(),
            initializer: initializer.as_ref()
                .map(|init| self.visit_node(init))
                .transpose()?
                .map(Box::new),
            type_annotation: type_annotation.clone(),
        })
    }

    fn visit_assignment(
        &mut self,
        target: &ASTNode,
        value: &ASTNode,
    ) -> Result<Self::Output> {
        Ok(ASTNode::Assignment {
            target: Box::new(self.visit_node(target)?),
            value: Box::new(self.visit_node(value)?),
        })
    }

    fn visit_return(
        &mut self,
        value: &Option<ASTNode>,
    ) -> Result<Self::Output> {
        Ok(ASTNode::Return(
            value.as_ref()
                .map(|v| self.visit_node(v))
                .transpose()?
                .map(Box::new)
        ))
    }

    // TODO: Implement other visitor methods...
}
