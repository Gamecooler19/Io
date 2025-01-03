use std::{fmt, io};

pub type Result<T> = std::result::Result<T, IoError>;

#[derive(Debug)]
pub enum ErrorKind {
    TypeError,
    RuntimeError,
    SyntaxError,
    LexerError,
    ParserError,
    CodegenError,
    NetworkError,
    Io,
    VerificationError,
    BuilderError,
    IntrinsicError,
    LinkageError,
    ParseError,
    TypeMismatch,
    DivisionByZero,
    ModuleNotFound,
    CircularDependency,
    ValidationError,
    ConcurrencyError,
}

#[derive(Debug)]
pub struct IoError {
    kind: ErrorKind,
    message: String,
}

impl IoError {
    pub fn lexer_error(position: usize, message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::LexerError,
            message: format!("Lexer error at position {}: {}", position, message.into()),
        }
    }

    pub fn parser_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::ParserError,
            message: message.into(),
        }
    }

    pub fn type_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::TypeError,
            message: message.into(),
        }
    }

    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::RuntimeError,
            message: message.into(),
        }
    }

    pub fn codegen_error(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::CodegenError,
            message: message.into(),
        }
    }

    // Add missing error variants
    pub fn stack_overflow() -> Self {
        Self {
            kind: ErrorKind::RuntimeError,
            message: "Stack overflow".into(),
        }
    }

    pub fn out_of_memory() -> Self {
        Self {
            kind: ErrorKind::RuntimeError,
            message: "Out of memory".into(),
        }
    }

    pub fn deadlock(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::RuntimeError,
            message: format!("Deadlock: {}", msg.into()),
        }
    }
}

impl From<io::Error> for IoError {
    fn from(err: io::Error) -> Self {
        IoError {
            kind: ErrorKind::Io,
            message: err.to_string(),
        }
    }
}

impl From<inkwell::builder::BuilderError> for IoError {
    fn from(err: inkwell::builder::BuilderError) -> Self {
        Self {
            kind: ErrorKind::BuilderError,
            message: err.to_string(),
        }
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IoError {}
