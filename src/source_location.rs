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
        let line_info = Self::compute_line_info(file);
        let (line, column) = Self::calculate_position(token_position, &line_info);
        
        Self {
            file: file.to_string(),
            directory: std::path::Path::new(file)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or(".")
                .to_string(),
            line,
            column,
        }
    }

    fn compute_line_info(content: &str) -> LineInfo {
        let mut line_starts = vec![0];
        let mut pos = 0;

        for c in content.chars() {
            pos += 1;
            if c == '\n' {
                line_starts.push(pos);
            }
        }

        LineInfo {
            line_starts,
            content_length: content.len(),
        }
    }

    fn calculate_position(pos: usize, info: &LineInfo) -> (u32, u32) {
        // Check bounds
        if pos > info.content_length {
            return (0, 0);
        }

        // Binary search to find the line number
        let line_idx = match info.line_starts.binary_search(&pos) {
            Ok(exact) => exact,
            Err(insert) => insert - 1,
        };

        let line = (line_idx + 1) as u32;
        let column = (pos - info.line_starts[line_idx] + 1) as u32;

        (line, column)
    }

    pub fn format_location(&self) -> String {
        format!("{}:{},{}", self.file, self.line, self.column)
    }

    pub fn is_before(&self, other: &Self) -> bool {
        if self.file != other.file {
            return false;
        }
        self.line < other.line || (self.line == other.line && self.column < other.column)
    }
}

#[derive(Debug)]
struct LineInfo {
    line_starts: Vec<usize>,
    content_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location() {
        let content = "line1\nline2\nline3";
        let loc = SourceLocation::from_token(content, 7); // Position after "line1\n"
        assert_eq!(loc.line, 2);
        assert_eq!(loc.column, 1);
    }

    #[test]
    fn test_empty_file() {
        let loc = SourceLocation::from_token("", 0);
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 1);
    }

    #[test]
    fn test_position_at_end() {
        let content = "line1\nline2";
        let loc = SourceLocation::from_token(content, 11); // Last position
        assert_eq!(loc.line, 2);
        assert_eq!(loc.column, 6);
    }
}
