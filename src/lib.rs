pub mod ast;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod stdlib;
pub mod symbol_table;
pub mod token;
pub mod types;

use crate::error::IoError;
pub type Result<T> = std::result::Result<T, IoError>;

// Re-export commonly used types
pub use ast::ASTNode;
pub use token::Token;
pub use types::Type;
