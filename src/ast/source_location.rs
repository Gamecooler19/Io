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
