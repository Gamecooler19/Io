#[derive(Debug, Clone, PartialEq)]
pub struct SourceLocation {
    pub file: String,
    pub directory: String,
    pub line: u32,
    pub column: u32,
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self {
            file: String::from("unknown"),
            directory: String::from("."),
            line: 0,
            column: 0,
        }
    }
}

impl SourceLocation {
    pub fn new(file: String, directory: String, line: u32, column: u32) -> Self {
        Self {
            file,
            directory,
            line,
            column,
        }
    }

    pub fn from_token(file: &str, token_position: usize) -> Self {
        //TODO: Simple implementation - could be improved with actual line/column tracking
        Self {
            file: file.to_string(),
            directory: ".".to_string(),
            line: (token_position / 80) as u32 + 1, // Assume 80 chars per line
            column: (token_position % 80) as u32 + 1,
        }
    }
}
