#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub position: usize,
} // Added missing closing brace

impl Token {
    pub fn new(kind: TokenKind, lexeme: String, position: usize) -> Self {
        Self {
            kind,
            lexeme,
            position,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    // Single-character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    Colon,
    Arrow,

    // One or two character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Keywords
    And,
    Or,
    If,
    Else,
    True,
    False,
    Function,
    Let,
    Return,
    While,
    For,
    Break,
    Continue,
    Async,
    Await,

    // Literals
    Identifier,
    String,
    Number,

    EOF,
}
