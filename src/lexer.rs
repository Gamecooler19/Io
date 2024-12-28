use crate::{
    token::{Token, TokenKind},
    IoError, Result,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alpha1, alphanumeric1, multispace1},
    combinator::{map, recognize},
    sequence::pair,
    IResult,
};

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while !self.input.is_empty() {
            // Skip whitespace
            if let Ok((remaining, _)) = multispace1::<&str, ()>(self.input) {
                let consumed = self.input.len() - remaining.len();
                self.position += consumed;
                self.input = remaining;
                continue;
            }

            // Try to match a token
            let token = self.next_token()?;
            tokens.push(token);
        }

        // Add EOF token
        tokens.push(Token::new(TokenKind::EOF, String::from(""), self.position));
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token> {
        // Match keywords and identifiers
        if let Ok((remaining, ident)) = self.identifier(self.input) {
            let token = match ident.as_str() {
                "fn" => Token::new(TokenKind::Function, ident, self.position),
                "let" => Token::new(TokenKind::Let, ident, self.position),
                "return" => Token::new(TokenKind::Return, ident, self.position),
                // TODO: ...existing code...
                _ => Token::new(TokenKind::Identifier, ident, self.position),
            };
            let consumed = self.input.len() - remaining.len();
            self.position += consumed;
            self.input = remaining;
            return Ok(token);
        }

        // Handle unrecognized input
        Err(IoError::lexer_error(
            self.position,
            format!(
                "Unexpected character: {}",
                self.input.chars().next().unwrap_or('\0')
            ),
        ))
    }

    fn identifier<'b>(&self, input: &'b str) -> IResult<&'b str, String> {
        map(
            recognize(pair(
                alt((alpha1, tag("_"))),
                take_while1(|c: char| c.is_alphanumeric() || c == '_'),
            )),
            String::from,
        )(input)
    }
}
