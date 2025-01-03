pub mod ast;
pub mod codegen;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod stdlib;
pub mod token; // Add token module
pub mod types;
pub mod validation;
pub mod visitor; // Add visitor module

// Export common types to avoid import conflicts
pub use ast::{ASTNode, Expression, Function, Module, Parameter, Statement};
pub use error::IoError;
pub type Result<T> = std::result::Result<T, IoError>;

// Re-export debug info types
pub use codegen::debug::{DebugInfo, SourceLocation};

// Re-export visitor trait
pub use visitor::{Visitable, Visitor};

mod prelude;
pub use prelude::*;
