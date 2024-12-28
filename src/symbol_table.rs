use crate::error::IoError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Symbol {
    Variable {
        name: String,
        type_name: String,
        mutable: bool,
    },
    Function {
        name: String,
        params: Vec<String>,
        return_type: Option<String>,
        is_async: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Box<Scope>) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn define(&mut self, name: String, symbol: Symbol) -> Result<(), IoError> {
        if self.symbols.contains_key(&name) {
            return Err(IoError::type_error(format!(
                "Symbol '{}' already defined in current scope",
                name
            )));
        }
        self.symbols.insert(name, symbol);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.lookup(name)))
    }
}
