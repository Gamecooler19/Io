use crate::error::{IoError, Result};
use inkwell::types::{BasicType, BasicTypeEnum};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Void,
    Bool,
    Int(u32),   // bit width
    Float(u32), // bit width
    Pointer(Box<Type>),
    Array(Box<Type>, Option<u32>),
    Struct(Vec<Type>, bool), // (fields, packed)
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        varargs: bool,
    },
    Vector(Box<Type>, u32),
    Opaque(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub attributes: TypeAttributes,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeAttributes {
    pub volatile: bool,
    pub atomic: bool,
    pub address_space: Option<u32>,
}

pub struct TypeRegistry<'ctx> {
    types: HashMap<String, BasicTypeEnum<'ctx>>,
    opaque_types: HashMap<String, inkwell::types::StructType<'ctx>>,
}

impl<'ctx> TypeRegistry<'ctx> {
    pub fn new() -> Self {
        Self {
            types: HashMap::new(),
            opaque_types: HashMap::new(),
        }
    }

    pub fn register_type(&mut self, name: &str, ty: BasicTypeEnum<'ctx>) -> Result<()> {
        if self.types.insert(name.to_string(), ty).is_some() {
            Err(IoError::type_error(format!(
                "Type {} already registered",
                name
            )))
        } else {
            Ok(())
        }
    }

    pub fn register_opaque(
        &mut self,
        name: &str,
        ty: inkwell::types::StructType<'ctx>,
    ) -> Result<()> {
        if self.opaque_types.insert(name.to_string(), ty).is_some() {
            Err(IoError::type_error(format!(
                "Opaque type {} already registered",
                name
            )))
        } else {
            Ok(())
        }
    }

    pub fn get_type(&self, name: &str) -> Option<BasicTypeEnum<'ctx>> {
        self.types.get(name).copied()
    }

    pub fn get_opaque(&self, name: &str) -> Option<inkwell::types::StructType<'ctx>> {
        self.opaque_types.get(name).copied()
    }
}

impl Type {
    pub fn void() -> Self {
        Self {
            kind: TypeKind::Void,
            attributes: TypeAttributes::default(),
        }
    }

    pub fn bool() -> Self {
        Self {
            kind: TypeKind::Bool,
            attributes: TypeAttributes::default(),
        }
    }

    pub fn i8() -> Self {
        Self {
            kind: TypeKind::Int(8),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn i32() -> Self {
        Self {
            kind: TypeKind::Int(32),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn i64() -> Self {
        Self {
            kind: TypeKind::Int(64),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn f32() -> Self {
        Self {
            kind: TypeKind::Float(32),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn f64() -> Self {
        Self {
            kind: TypeKind::Float(64),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn ptr(ty: Type) -> Self {
        Self {
            kind: TypeKind::Pointer(Box::new(ty)),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn array(ty: Type, size: Option<u32>) -> Self {
        Self {
            kind: TypeKind::Array(Box::new(ty), size),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn function(params: Vec<Type>, return_type: Type, varargs: bool) -> Self {
        Self {
            kind: TypeKind::Function {
                params,
                return_type: Box::new(return_type),
                varargs,
            },
            attributes: TypeAttributes::default(),
        }
    }

    pub fn struct_type(fields: Vec<Type>, packed: bool) -> Self {
        Self {
            kind: TypeKind::Struct(fields, packed),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn opaque(name: &str) -> Self {
        Self {
            kind: TypeKind::Opaque(name.to_string()),
            attributes: TypeAttributes::default(),
        }
    }

    pub fn with_volatile(mut self) -> Self {
        self.attributes.volatile = true;
        self
    }

    pub fn with_atomic(mut self) -> Self {
        self.attributes.atomic = true;
        self
    }

    pub fn with_address_space(mut self, space: u32) -> Self {
        self.attributes.address_space = Some(space);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    #[test]
    fn test_type_creation() {
        let context = Context::create();
        let i32_type = Type::i32();
        let ptr_type = Type::ptr(i32_type.clone());
        let array_type = Type::array(i32_type, Some(10));

        let llvm_i32 = i32_type.into_llvm(&context);
        assert!(llvm_i32.is_int_type());

        let llvm_ptr = ptr_type.into_llvm(&context);
        assert!(llvm_ptr.is_pointer_type());

        let llvm_array = array_type.into_llvm(&context);
        assert!(llvm_array.is_array_type());
    }

    #[test]
    fn test_type_registry() {
        let context = Context::create();
        let mut registry = TypeRegistry::new();

        let i32_type = Type::i32().into_llvm(&context);
        registry.register_type("i32", i32_type).unwrap();

        assert!(registry.get_type("i32").is_some());
        assert!(registry.get_type("nonexistent").is_none());
    }
}
