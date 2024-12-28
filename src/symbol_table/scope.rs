use std::collections::HashMap;
use crate::{error::IoError, Result};
use super::Symbol;

#[derive(Debug, Clone)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<Box<Scope>>,
    level: usize,
    is_loop: bool,
    is_async: bool,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            parent: None,
            level: 0,
            is_loop: false,
            is_async: false,
        }
    }

    pub fn with_parent(parent: Box<Scope>) -> Self {
        Self {
            symbols: HashMap::new(),
            level: parent.level + 1,
            parent: Some(parent),
            is_loop: false,
            is_async: false,
        }
    }

    pub fn define(&mut self, name: String, symbol: Symbol) -> Result<()> {
        if self.symbols.contains_key(&name) {
            return Err(IoError::validation_error(format!(
                "Symbol '{}' already defined in current scope",
                name
            )));
        }
        self.symbols.insert(name, symbol);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name).or_else(|| {
            self.parent.as_ref().and_then(|p| p.lookup(name))
        })
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.symbols.get_mut(name).or_else(|| {
            self.parent.as_mut().and_then(|p| p.lookup_mut(name))
        })
    }

    pub fn get_parent(&self) -> Option<Box<Scope>> {
        self.parent.clone()
    }

    pub fn in_loop(&self) -> bool {
        self.is_loop || self.parent.as_ref().map_or(false, |p| p.in_loop())
    }

    pub fn in_async_context(&self) -> bool {
        self.is_async || self.parent.as_ref().map_or(false, |p| p.in_async_context())
    }

    pub fn set_loop(&mut self) {
        self.is_loop = true;
    }

    pub fn set_async(&mut self) {
        self.is_async = true;
    }
}
