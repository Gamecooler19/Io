use std::sync::Arc;
use parking_lot::RwLock;
use crate::error::IoError;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ErrorContext {
    file: String,
    line: usize,
    column: usize,
    source_line: String,
}

#[derive(Debug)]
pub struct ErrorHandler {
    contexts: Arc<RwLock<Vec<ErrorContext>>>,
    error_count: Arc<RwLock<usize>>,
    recovery_strategy: Option<RecoveryStrategy>,
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(RwLock::new(Vec::new())),
            error_count: Arc::new(RwLock::new(0)),
            recovery_strategy: None,
        }
    }

    pub fn push_context(&self, context: ErrorContext) {
        self.contexts.write().push(context);
    }

    pub fn pop_context(&self) {
        self.contexts.write().pop();
    }

    pub fn handle_error(&self, error: IoError) -> IoError {
        let mut count = self.error_count.write();
        *count += 1;

        let contexts = self.contexts.read();
        if let Some(context) = contexts.last() {
            match &error {
                IoError::LexerError { position, message } => {
                    IoError::LexerError {
                        position: *position,
                        message: format!(
                            "{}\nIn file {} at line {}:{}\n{}\n{}^",
                            message,
                            context.file,
                            context.line,
                            context.column,
                            context.source_line,
                            " ".repeat(context.column)
                        ),
                    }
                }
                IoError::ParserError { span, message } => {
                    IoError::ParserError {
                        span: *span,
                        message: format!(
                            "{}\nIn file {} at line {}:{}\n{}\n{}^",
                            message,
                            context.file,
                            context.line,
                            context.column,
                            context.source_line,
                            " ".repeat(context.column)
                        ),
                    }
                }
                IoError::TypeError { expected, found, span } => {
                    IoError::TypeError {
                        expected: expected.clone(),
                        found: found.clone(),
                        span: *span,
                        message: format!(
                            "Type mismatch at {}:{}:{}\nExpected: {}\nFound: {}\n{}\n{}^",
                            context.file,
                            context.line,
                            context.column,
                            expected,
                            found,
                            context.source_line,
                            " ".repeat(context.column)
                        ),
                    }
                }
                IoError::RuntimeError { message, stack_trace } => {
                    IoError::RuntimeError {
                        message: format!(
                            "{}\nAt {}:{}:{}\n{}\n{}^",
                            message,
                            context.file,
                            context.line,
                            context.column,
                            context.source_line,
                            " ".repeat(context.column)
                        ),
                        stack_trace: stack_trace.clone(),
                    }
                }
                IoError::UnresolvedSymbol { name, scope } => {
                    IoError::UnresolvedSymbol {
                        name: name.clone(),
                        scope: scope.clone(),
                        message: format!(
                            "Unresolved symbol '{}' in scope '{}'\nAt {}:{}:{}\n{}\n{}^",
                            name,
                            scope,
                            context.file,
                            context.line,
                            context.column,
                            context.source_line,
                            " ".repeat(context.column)
                        ),
                    }
                }
                IoError::ValidationError { code, details } => {
                    IoError::ValidationError {
                        code: *code,
                        details: format!(
                            "{}\nAt {}:{}:{}\n{}\n{}^",
                            details,
                            context.file,
                            context.line,
                            context.column,
                            context.source_line,
                            " ".repeat(context.column)
                        ),
                    }
                }
            }
        } else {
            error
        }
    }

    pub fn handle_error_chain(&self, error: IoError, chain: &[IoError]) -> IoError {
        let mut enhanced_message = String::new();
        let contexts = self.contexts.read();

        // Build error chain message
        for (i, err) in chain.iter().enumerate() {
            enhanced_message.push_str(&format!("\nCaused by ({}):\n  {}", i + 1, err));
        }

        if let Some(context) = contexts.last() {
            match &error {
                IoError::LexerError { position, message } => {
                    let location = self.get_error_location(context, *position);
                    IoError::LexerError {
                        position: *position,
                        message: format!(
                            "{}\n{}\nAt {}:{}:{}\n{}\n{}^{}",
                            message,
                            enhanced_message,
                            context.file,
                            location.line,
                            location.column,
                            self.get_context_lines(context, location.line),
                            " ".repeat(location.column),
                            if chain.is_empty() { "" } else { "\nError chain:" }
                        ),
                    }
                }
                IoError::ParserError { span, message } => {
                    let location = self.get_error_location(context, span.start);
                    IoError::ParserError {
                        span: *span,
                        message: format!(
                            "{}\n{}\nAt {}:{}:{}\n{}\n{}^",
                            message,
                            enhanced_message,
                            context.file,
                            location.line,
                            location.column,
                            self.get_context_lines(context, location.line),
                            " ".repeat(location.column)
                        ),
                    }
                }
                IoError::TypeError { expected, found, span } => {
                    let location = self.get_error_location(context, span.start);
                    IoError::TypeError {
                        expected: expected.clone(),
                        found: found.clone(),
                        span: *span,
                        message: format!(
                            "Type mismatch\n{}\nAt {}:{}:{}\n{}\n{}^",
                            enhanced_message,
                            context.file,
                            location.line,
                            location.column,
                            self.get_context_lines(context, location.line),
                            " ".repeat(location.column)
                        ),
                    }
                }
                IoError::RuntimeError { message, stack_trace } => {
                    IoError::RuntimeError {
                        message: format!(
                            "{}\n{}\nStack trace:\n{}",
                            message,
                            enhanced_message,
                            stack_trace.join("\n")
                        ),
                        stack_trace: stack_trace.clone(),
                    }
                }
                IoError::UnresolvedSymbol { name, scope } => {
                    IoError::UnresolvedSymbol {
                        name: name.clone(),
                        scope: scope.clone(),
                        message: format!(
                            "Symbol '{}' not found in scope '{}'\n{}",
                            name,
                            scope,
                            enhanced_message
                        ),
                    }
                }
                IoError::ValidationError { code, details } => {
                    IoError::ValidationError {
                        code: *code,
                        details: format!(
                            "Validation error {}: {}\n{}",
                            code,
                            details,
                            enhanced_message
                        ),
                    }
                }
            }
        } else {
            error
        }
    }

    pub fn get_error_location(&self, context: &ErrorContext, position: usize) -> ErrorLocation {
        let mut line = 1;
        let mut column = 0;
        let mut current_pos = 0;

        for (i, c) in context.source_line.chars().enumerate() {
            if current_pos == position {
                column = i;
                break;
            }
            if c == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
            current_pos += c.len_utf8();
        }

        ErrorLocation { line, column }
    }

    pub fn get_context_lines(&self, context: &ErrorContext, error_line: usize) -> String {
        let lines: Vec<&str> = context.source_line.lines().collect();
        let start = error_line.saturating_sub(2);
        let end = (error_line + 2).min(lines.len());
        let mut result = String::new();

        for i in start..end {
            result.push_str(&format!(
                "{:>4} | {}\n",
                i + 1,
                lines.get(i).unwrap_or(&"")
            ));
        }
        result
    }

    pub fn try_recover<T>(
        &self,
        error: IoError,
        recovery_fn: impl FnOnce() -> Result<T, IoError>,
    ) -> Result<T, IoError> {
        match &self.recovery_strategy {
            Some(strategy) => {
                let action = self.determine_recovery_action(&error, strategy)?;
                match action {
                    RecoveryAction::SkipToken => {
                        log::warn!("Skipping token due to error: {}", error);
                        recovery_fn()
                    }
                    RecoveryAction::SynchronizeTo(tokens) => {
                        log::warn!("Synchronizing to tokens: {:?}", tokens);
                        recovery_fn()
                    }
                    RecoveryAction::UseType(default_type) => {
                        log::warn!("Using default type: {:?}", default_type);
                        recovery_fn()
                    }
                    RecoveryAction::Abort => Err(error),
                }
            }
            None => Err(error),
        }
    }

    fn determine_recovery_action(
        &self,
        error: &IoError,
        strategy: &RecoveryStrategy,
    ) -> Result<RecoveryAction, IoError> {
        match (error, strategy) {
            (IoError::LexerError { .. }, RecoveryStrategy::SkipToNextToken) => {
                Ok(RecoveryAction::SkipToken)
            }
            (IoError::ParserError { .. }, RecoveryStrategy::SynchronizeTo(tokens)) => {
                Ok(RecoveryAction::SynchronizeTo(tokens.clone()))
            }
            (IoError::TypeError { .. }, RecoveryStrategy::UseDefaultType(default_type)) => {
                Ok(RecoveryAction::UseType(default_type.clone()))
            }
            _ => Err(IoError::runtime_error("No suitable recovery action found")),
        }
    }

    pub fn with_context<T, F>(&self, context: ErrorContext, f: F) -> Result<T, IoError>
    where
        F: FnOnce() -> Result<T, IoError>,
    {
        self.push_context(context);
        let result = f();
        self.pop_context();
        result
    }

    pub fn error_count(&self) -> usize {
        *self.error_count.read()
    }

    pub fn with_recovery_strategy(&mut self, strategy: RecoveryStrategy) -> &mut Self {
        self.recovery_strategy = Some(strategy);
        self
    }

    pub fn handle_error_with_recovery(&self, error: IoError) -> Result<RecoveryAction, IoError> {
        let mut count = self.error_count.write();
        *count += 1;

        match &self.recovery_strategy {
            Some(strategy) => match (&error, strategy) {
                (IoError::LexerError { .. }, RecoveryStrategy::SkipToNextToken) => {
                    Ok(RecoveryAction::SkipToken)
                }
                (IoError::ParserError { .. }, RecoveryStrategy::SynchronizeTo(tokens)) => {
                    Ok(RecoveryAction::SynchronizeTo(tokens.clone()))
                }
                (IoError::TypeError { .. }, RecoveryStrategy::UseDefaultType(default_type)) => {
                    Ok(RecoveryAction::UseType(default_type.clone()))
                }
                _ => Err(error),
            },
            None => Err(error),
        }
    }

    pub fn create_error_report(&self) -> ErrorReport {
        let contexts = self.contexts.read();
        let count = *self.error_count.read();

        ErrorReport {
            total_errors: count,
            contexts: contexts.clone(),
            timestamp: std::time::SystemTime::now(),
            error_categories: self.categorize_errors(),
        }
    }

    fn categorize_errors(&self) -> HashMap<ErrorCategory, usize> {
        let mut categories = HashMap::new();
        let contexts = self.contexts.read();

        for context in contexts.iter() {
            let category = ErrorCategory::from_context(context);
            *categories.entry(category).or_insert(0) += 1;
        }

        categories
    }
}

#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    SkipToNextToken,
    SynchronizeTo(Vec<TokenKind>),
    UseDefaultType(Type),
    Abort,
}

#[derive(Debug, Clone)]
pub enum RecoveryAction {
    SkipToken,
    SynchronizeTo(Vec<TokenKind>),
    UseType(Type),
    Abort,
}

#[derive(Debug, Clone)]
pub struct ErrorReport {
    total_errors: usize,
    contexts: Vec<ErrorContext>,
    timestamp: std::time::SystemTime,
    error_categories: HashMap<ErrorCategory, usize>,
}

// Add error diagnostics
impl ErrorReport {
    pub fn print_summary(&self) -> String {
        use colored::*;
        let mut summary = String::new();

        summary.push_str(&format!("\nError Report ({})\n", 
            self.timestamp.duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        summary.push_str("==============\n");

        // Print error categories
        summary.push_str("\nError Categories:\n");
        for (category, count) in &self.error_categories {
            summary.push_str(&format!("  {}: {} errors\n",
                match category {
                    ErrorCategory::Syntax => "Syntax".red(),
                    ErrorCategory::Type => "Type".yellow(),
                    ErrorCategory::Runtime => "Runtime".red(),
                    ErrorCategory::System => "System".bright_red(),
                },
                count
            ));
        }

        // Print error contexts
        if !self.contexts.is_empty() {
            summary.push_str("\nError Contexts:\n");
            for (i, context) in self.contexts.iter().enumerate() {
                summary.push_str(&format!("{}. In file {} at line {}:{}\n",
                    i + 1,
                    context.file.blue(),
                    context.line.to_string().yellow(),
                    context.column
                ));
                summary.push_str(&format!("   {}\n", context.source_line));
                summary.push_str(&format!("   {}{}\n",
                    " ".repeat(context.column),
                    "^".red()
                ));
            }
        }

        summary
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ErrorCategory {
    Syntax,
    Type,
    Runtime,
    System,
}

impl ErrorCategory {
    fn from_context(context: &ErrorContext) -> Self {
        // Determine category based on error context
        if context.source_line.contains("type") {
            ErrorCategory::Type
        } else if context.source_line.contains("fn") || context.source_line.contains("let") {
            ErrorCategory::Syntax
        } else {
            ErrorCategory::Runtime
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorLocation {
    pub line: usize,
    pub column: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_handler_recovery() {
        let mut handler = ErrorHandler::new();
        handler.with_recovery_strategy(RecoveryStrategy::SkipToNextToken);

        let result = handler.handle_error_with_recovery(
            IoError::LexerError {
                position: 0,
                message: "test error".to_string(),
            }
        );

        assert!(matches!(result, Ok(RecoveryAction::SkipToken)));
    }

    #[test]
    fn test_error_context_tracking() {
        let handler = ErrorHandler::new();
        let context = ErrorContext {
            file: "test.io".to_string(),
            line: 1,
            column: 5,
            source_line: "let x = 42;".to_string(),
        };

        handler.push_context(context);
        assert_eq!(handler.error_count(), 0);

        let error = IoError::LexerError {
            position: 5,
            message: "unexpected token".to_string(),
        };

        let enhanced = handler.handle_error(error);
        assert!(matches!(enhanced, IoError::LexerError { .. }));
    }
}
