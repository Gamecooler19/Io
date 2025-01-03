pub mod checker;

use crate::{error::IoError, Result};
use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
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
    Unit,  // Added Unit variant
    Int,   // Added Int variant
    Float, // Added Float variant
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
            Type::Int | Type::I32 => context.i32_type().into(),
            Type::Float | Type::F32 => context.f32_type().into(),
            Type::Unit | Type::Void => context.void_type().into(),
            Type::I8 => context.i8_type().into(),
            Type::I64 => context.i64_type().into(),
            Type::F64 => context.f64_type().into(),
            Type::Bool => context.bool_type().into(),
            Type::String => context
                .i8_type()
                .ptr_type(inkwell::AddressSpace::default())
                .into(),
            Type::Array { elem_type, size } => {
                let elem_ty = elem_type.to_llvm_type(context);
                context
                    .struct_type(&[elem_ty], false)
                    .array_type(*size as u32)
                    .as_basic_type_enum()
            }
            Type::Function {
                params,
                return_type,
                ..
            } => {
                let ret_type = return_type.to_llvm_type(context);
                let param_types: Vec<_> = params
                    .iter()
                    .map(|t| t.to_llvm_type(context).into())
                    .collect();
                ret_type
                    .fn_type(&param_types, false)
                    .ptr_type(AddressSpace::default())
                    .as_basic_type_enum()
            }
            Type::Struct { fields, .. } => {
                let field_types: Vec<_> = fields
                    .iter()
                    .map(|(_, t)| t.to_llvm_type(context))
                    .collect();
                context.struct_type(&field_types, false).into()
            }
        }
    }

    pub fn fn_type<'a>(
        &self,
        param_types: &[BasicTypeEnum<'a>],
        is_var_args: bool,
        context: &'a Context,
    ) -> inkwell::types::FunctionType<'a> {
        match self {
            Type::Function { return_type, .. } => {
                let ret_ty = return_type.to_llvm_type(context);
                let param_types: Vec<BasicMetadataTypeEnum> = param_types
                    .iter()
                    .map(|t| t.as_basic_type_enum().into())
                    .collect();
                ret_ty.fn_type(&param_types, is_var_args)
            }
            _ => panic!("Called fn_type on non-function type"),
        }
    }

    pub fn array_type<'a>(&self, size: u32, context: &'a Context) -> inkwell::types::ArrayType<'a> {
        match self {
            Type::Array { elem_type, .. } => {
                let elem_ty = elem_type.to_llvm_type(context);
                elem_ty.array_type(size)
            }
            _ => panic!("Called array_type on non-array type"),
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
                if (*is_async) {
                    write!(f, "async ")?;
                }
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if (i > 0) {
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
                    if (i > 0) {
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

impl SomeType {
    pub fn some_method(&self, context: &Context) -> Result<inkwell::types::FunctionType<'_>> {
        let return_type = self.get_return_type()?;
        let ret_ty = return_type.to_llvm_type(context);

        let param_types: Vec<BasicTypeEnum> = self
            .parameters
            .iter()
            .map(|param| param.get_type().to_llvm_type(context))
            .collect();

        Ok(ret_ty.fn_type(&param_types, self.is_variadic))
    }

    pub fn another_method(
        &self,
        elem_type: &Type,
        context: &Context,
    ) -> Result<inkwell::types::ArrayType<'_>> {
        let elem_ty = elem_type.to_llvm_type(context);

        // Validate array size
        if self.size == 0 {
            return Err(IoError::type_error("Array size must be greater than 0"));
        }

        // Create array type with proper alignment
        let array_type = match elem_ty {
            BasicTypeEnum::IntType(int_ty) => int_ty.array_type(self.size as u32),
            BasicTypeEnum::FloatType(float_ty) => float_ty.array_type(self.size as u32),
            BasicTypeEnum::PointerType(ptr_ty) => ptr_ty.array_type(self.size as u32),
            _ => return Err(IoError::type_error("Unsupported element type for array")),
        };

        Ok(array_type)
    }

    fn get_return_type(&self) -> Result<Type> {
        self.return_type
            .clone()
            .ok_or_else(|| IoError::type_error("Return type not specified"))
    }
}
