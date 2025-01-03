use super::{Expression, Parameter, Statement, Type};

#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<Function>,
    pub declarations: Vec<Declaration>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub enum Declaration {
    Function(Function),
    Struct(StructDef),
    Import(Import),
    Global(Global),
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Global {
    pub name: String,
    pub r#type: Type,
    pub value: Option<Expression>,
}
