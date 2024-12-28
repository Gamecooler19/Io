use std::{fmt, io};

#[derive(Debug)]
pub enum IoError {
    LexerError { position: usize, message: String },
    ParserError { message: String },
    TypeError { message: String },
    RuntimeError { message: String },
    VerificationError { message: String },
    NetworkError(String),
    Io(io::Error),
}

impl IoError {
    pub fn lexer_error(position: usize, message: impl Into<String>) -> Self {
        Self::LexerError {
            position,
            message: message.into(),
        }
    }

    pub fn parser_error(message: impl Into<String>) -> Self {
        Self::ParserError {
            message: message.into(),
        }
    }

    pub fn type_error(message: impl Into<String>) -> Self {
        Self::TypeError {
            message: message.into(),
        }
    }

    pub fn runtime_error(message: impl Into<String>) -> Self {
        Self::RuntimeError {
            message: message.into(),
        }
    }

    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::VerificationError {
            message: message.into(),
        }
    }

    pub fn macro_error(message: impl Into<String>) -> Self {
        Self::RuntimeError {
            message: message.into(),
        }
    }

    pub fn debug_error(message: impl Into<String>) -> Self {
        Self::RuntimeError {
            message: message.into(),
        }
    }
}

impl From<io::Error> for IoError {
    fn from(err: io::Error) -> Self {
        IoError::Io(err)
    }
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoError::LexerError { position, message } => {
                write!(f, "Lexer error at position {}: {}", position, message)
            }
            IoError::ParserError { message } => write!(f, "Parser error: {}", message),
            IoError::TypeError { message } => write!(f, "Type error: {}", message),
            IoError::RuntimeError { message } => write!(f, "Runtime error: {}", message),
            IoError::VerificationError { message } => write!(f, "Verification error: {}", message),
            IoError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            IoError::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for IoError {}
