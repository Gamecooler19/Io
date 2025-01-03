#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
    Bool,
    String,
    Void,
    Array(Box<Type>),
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
    },
    Struct {
        name: String,
        fields: Vec<(String, Type)>,
    },
    Pointer(Box<Type>),
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub r#type: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Array(Vec<Literal>),
    Unit,
}
