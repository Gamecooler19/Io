use crate::error::Result;
use crate::token::{Token, TokenKind};
use std::iter::Peekable;

pub struct Parser<I: Iterator<Item = Token>> {
    tokens: Peekable<I>,
    current: Option<Token>,
}

impl<I: Iterator<Item = Token>> Parser<I> {
    pub fn new(tokens: I) -> Self {
        let mut parser = Self {
            tokens: tokens.peekable(),
            current: None,
        };
        parser.advance();
        parser
    }

    fn advance(&mut self) -> Option<Token> {
        self.current = self.tokens.next();
        self.current.clone()
    }

    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek()
    }

    fn match_token(&mut self, kinds: &[TokenKind]) -> bool {
        if let Some(token) = self.peek() {
            if kinds.contains(&token.kind) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn expect_token(&mut self, kind: TokenKind) -> Result<Token> {
        if let Some(token) = self.advance() {
            if token.kind == kind {
                Ok(token)
            } else {
                Err(format!("Expected {:?}, got {:?}", kind, token.kind).into())
            }
        } else {
            Err("Unexpected end of input".into())
        }
    }

    // Add parsing methods for each AST node type...
}
