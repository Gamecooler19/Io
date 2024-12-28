use crate::error::IoError;
use colored::*;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub hints: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            location: None,
            hints: Vec::new(),
        }
    }

    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }

    pub fn report(&self, source_map: &SourceMap) -> String {
        let mut output = String::new();

        let prefix = match self.level {
            DiagnosticLevel::Error => "error".red().bold(),
            DiagnosticLevel::Warning => "warning".yellow().bold(),
            DiagnosticLevel::Info => "info".blue().bold(),
        };

        output.push_str(&format!("{}: {}\n", prefix, self.message));

        if let Some(location) = &self.location {
            if let Some(source) = source_map.get_source(&location.file) {
                let line = source.get_line(location.line);
                output.push_str(&format!(
                    " --> {}:{}:{}\n",
                    location.file.display(),
                    location.line,
                    location.column
                ));
                output.push_str(&format!("  |\n"));
                output.push_str(&format!("{:3} | {}\n", location.line, line));
                output.push_str(&format!(
                    "  | {}{}",
                    " ".repeat(location.column),
                    "^".repeat(location.length).green()
                ));
            }
        }

        for hint in &self.hints {
            output.push_str(&format!("\nhelp: {}", hint.blue()));
        }

        output
    }
}

pub struct SourceMap {
    sources: HashMap<PathBuf, Source>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, path: PathBuf, content: String) {
        self.sources.insert(path, Source::new(content));
    }

    pub fn get_source(&self, path: &PathBuf) -> Option<&Source> {
        self.sources.get(path)
    }
}

pub struct Source {
    content: String,
    lines: Vec<usize>, // Line start positions
}

impl Source {
    pub fn new(content: String) -> Self {
        let mut lines = vec![0];
        for (i, c) in content.chars().enumerate() {
            if c == '\n' {
                lines.push(i + 1);
            }
        }
        Self { content, lines }
    }

    pub fn get_line(&self, line: usize) -> &str {
        let start = self.lines[line - 1];
        let end = self.lines.get(line).copied().unwrap_or(self.content.len());
        &self.content[start..end].trim_end()
    }
}
