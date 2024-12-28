pub mod checker;

use crate::{error::IoError, Result};
use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::AddressSpace;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I8,
    I32,
    I64,
    F32,
    F64,
    Bool,
    Void,
    String,
    Array {
        elem_type: Box<Type>,
        size: usize,
    },
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        is_async: bool,
    },
    Struct {
        name: String,
        fields: Vec<(String, Type)>,
    },
}

impl Type {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "int" => Ok(Type::I32),
            "float" => Ok(Type::F32),
            "string" => Ok(Type::String),
            "bool" => Ok(Type::Bool),
            "unit" => Ok(Type::Void),
            s if s.starts_with("array<") => {
                let inner = s[6..s.len() - 1].trim();
                Ok(Type::Array {
                    elem_type: Box::new(Type::from_str(inner)?),
                    size: 0,
                })
            }
            _ => Err(IoError::type_error(format!("Unknown type: {}", s))),
        }
    }

    pub fn to_llvm_type<'ctx>(&self, context: &'ctx Context) -> BasicTypeEnum<'ctx> {
        match self {
            Type::I8 => context.i8_type().into(),
            Type::I32 => context.i32_type().into(),
            Type::I64 => context.i64_type().into(),
            Type::F32 => context.f32_type().into(),
            Type::F64 => context.f64_type().into(),
            Type::Bool => context.bool_type().into(),
            Type::String => context.i8_type().ptr_type(AddressSpace::default()).into(),
            Type::Array { elem_type, size } => {
                let elem_ty = elem_type.to_llvm_type(context);
                context
                    .get_struct_type(&[elem_ty])
                    .array_type(*size as u32)
                    .into()
            }
            Type::Function {
                params,
                return_type,
                ..
            } => {
                let ret_type = return_type.to_llvm_type(context);
                let param_types: Vec<_> = params.iter().map(|t| t.to_llvm_type(context)).collect();
                context
                    .get_struct_type(&[ret_type])
                    .func_type(&param_types, false)
                    .into()
            }
            Type::Struct { fields, .. } => {
                let field_types: Vec<_> = fields
                    .iter()
                    .map(|(_, t)| t.to_llvm_type(context))
                    .collect();
                context.struct_type(&field_types, false).into()
            }
            Type::Void => context.void_type().ptr_type(AddressSpace::default()).into(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::I32 => write!(f, "int32"),
            Type::I64 => write!(f, "int64"),
            Type::F32 => write!(f, "float32"),
            Type::F64 => write!(f, "float64"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::Void => write!(f, "unit"),
            Type::Function {
                params,
                return_type,
                is_async,
            } => {
                if *is_async {
                    write!(f, "async ")?;
                }
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::Array { elem_type, size } => write!(f, "array<{}; {}>", elem_type, size),
            Type::Struct { fields, name } => {
                write!(f, "struct {} {{ ", name)?;
                for (i, (field_name, field_type)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", field_name, field_type)?;
                }
                write!(f, " }}")
            }
            _ => write!(f, "{:?}", self),
        }
    }
}

#[derive(Debug)]
pub struct TypeContext {
    types: HashMap<String, Type>,
    generics: Vec<String>,
}

impl TypeContext {
    pub fn new() -> Self {
        let mut types = HashMap::new();
        // Register built-in types
        types.insert("int".to_string(), Type::I32);
        types.insert("float".to_string(), Type::F32);
        types.insert("string".to_string(), Type::String);
        types.insert("bool".to_string(), Type::Bool);
        types.insert("unit".to_string(), Type::Void);

        Self {
            types,
            generics: Vec::new(),
        }
    }

    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.types.get(name)
    }

    pub fn register_type(&mut self, name: String, ty: Type) -> Result<(), IoError> {
        if self.types.contains_key(&name) {
            return Err(IoError::type_error(format!("Type {} already exists", name)));
        }
        self.types.insert(name, ty);
        Ok(())
    }
}
