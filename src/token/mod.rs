use crate::Result;

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

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub value: String,
    pub position: usize,
    pub literal: Option<LiteralValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    String(String),
    Number(f64),
    Boolean(bool),
}

impl Token {
    pub fn new(kind: TokenKind, value: String, position: usize) -> Self {
        Self {
            kind,
            value,
            position,
            literal: None,
        }
    }

    pub fn with_literal(
        kind: TokenKind,
        value: String,
        position: usize,
        literal: LiteralValue,
    ) -> Self {
        Self {
            kind,
            value,
            position,
            literal: Some(literal),
        }
    }
}
