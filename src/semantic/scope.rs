use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub type_name: String,
    pub mutable: bool,
    pub defined: bool,
}

#[derive(Debug)]
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

    pub fn with_parent(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, name: &str, type_name: &str, mutable: bool) -> Result<(), String> {
        if self.symbols.contains_key(name) {
            return Err(format!(
                "Symbol '{}' already defined in current scope",
                name
            ));
        }

        self.symbols.insert(
            name.to_string(),
            Symbol {
                name: name.to_string(),
                type_name: type_name.to_string(),
                mutable,
                defined: true,
            },
        );

        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        match self.symbols.get(name) {
            Some(symbol) => Some(symbol),
            None => match &self.parent {
                Some(parent) => parent.lookup(name),
                None => None,
            },
        }
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        if self.symbols.contains_key(name) {
            self.symbols.get_mut(name)
        } else {
            match &mut self.parent {
                Some(parent) => parent.lookup_mut(name),
                None => None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_definition() {
        let mut scope = Scope::new();
        assert!(scope.define("x", "int", false).is_ok());
        assert!(scope.define("x", "int", false).is_err());
    }

    #[test]
    fn test_scope_lookup() {
        let mut parent = Scope::new();
        parent.define("x", "int", false).unwrap();

        let mut child = Scope::with_parent(parent);
        child.define("y", "string", true).unwrap();

        assert!(child.lookup("x").is_some());
        assert!(child.lookup("y").is_some());
        assert!(child.lookup("z").is_none());
    }

    #[test]
    fn test_symbol_mutability() {
        let mut scope = Scope::new();
        scope.define("x", "int", true).unwrap();
        scope.define("y", "int", false).unwrap();

        assert!(scope.lookup("x").unwrap().mutable);
        assert!(!scope.lookup("y").unwrap().mutable);
    }
}
