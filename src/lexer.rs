use crate::{
    token::{Token, TokenKind},
    IoError, Result,
};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{alpha1, multispace1},
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
                "if" => Token::new(TokenKind::If, ident, self.position),
                "else" => Token::new(TokenKind::Else, ident, self.position),
                "while" => Token::new(TokenKind::While, ident, self.position),
                "for" => Token::new(TokenKind::For, ident, self.position),
                "break" => Token::new(TokenKind::Break, ident, self.position),
                "continue" => Token::new(TokenKind::Continue, ident, self.position),
                "true" => Token::new(TokenKind::Boolean(true), ident, self.position),
                "false" => Token::new(TokenKind::Boolean(false), ident, self.position),
                _ => Token::new(TokenKind::Identifier, ident, self.position),
            };
            self.advance(remaining);
            return Ok(token);
        }

        // Match numbers
        if let Ok((remaining, number)) = self.number(self.input) {
            let token = Token::new(TokenKind::Number, number, self.position);
            self.advance(remaining);
            return Ok(token);
        }

        // Match strings
        if let Ok((remaining, string)) = self.string(self.input) {
            let token = Token::new(TokenKind::String, string, self.position);
            self.advance(remaining);
            return Ok(token);
        }

        // Match operators and symbols
        match self.input.chars().next() {
            Some(ch) => {
                let (kind, len) = match ch {
                    '+' => (TokenKind::Plus, 1),
                    '-' => (TokenKind::Minus, 1),
                    '*' => (TokenKind::Star, 1),
                    '/' => (TokenKind::Slash, 1),
                    '=' => {
                        if self.input.starts_with("==") {
                            (TokenKind::EqualEqual, 2)
                        } else {
                            (TokenKind::Equal, 1)
                        }
                    }
                    '!' => {
                        if self.input.starts_with("!=") {
                            (TokenKind::BangEqual, 2)
                        } else {
                            (TokenKind::Bang, 1)
                        }
                    }
                    '<' => {
                        if self.input.starts_with("<=") {
                            (TokenKind::LessEqual, 2)
                        } else {
                            (TokenKind::Less, 1)
                        }
                    }
                    '>' => {
                        if self.input.starts_with(">=") {
                            (TokenKind::GreaterEqual, 2)
                        } else {
                            (TokenKind::Greater, 1)
                        }
                    }
                    '(' => (TokenKind::LeftParen, 1),
                    ')' => (TokenKind::RightParen, 1),
                    '{' => (TokenKind::LeftBrace, 1),
                    '}' => (TokenKind::RightBrace, 1),
                    '[' => (TokenKind::LeftBracket, 1),
                    ']' => (TokenKind::RightBracket, 1),
                    ',' => (TokenKind::Comma, 1),
                    '.' => (TokenKind::Dot, 1),
                    ';' => (TokenKind::Semicolon, 1),
                    ':' => (TokenKind::Colon, 1),
                    _ => {
                        return Err(IoError::lexer_error(
                            self.position,
                            format!("Unexpected character: {}", ch),
                        ))
                    }
                };

                let token = Token::new(kind, self.input[..len].to_string(), self.position);
                self.advance(&self.input[len..]);
                Ok(token)
            }
            None => Ok(Token::new(TokenKind::EOF, String::new(), self.position)),
        }
    }

    fn number<'b>(&self, input: &'b str) -> IResult<&'b str, String> {
        use nom::character::complete::{char, digit1};
        use nom::combinator::opt;
        use nom::sequence::tuple;

        let (input, (int_part, decimal_part)) =
            tuple((digit1, opt(tuple((char('.'), digit1)))))(input)?;

        let number = match decimal_part {
            Some(('.', decimal)) => format!("{}.{}", int_part, decimal),
            None => int_part.to_string(),
            Some(('\0'..='-', _)) | Some(('/', ..)) | Some(('\u{e000}'..='\u{10ffff}', _)) => {
                todo!()
            } // Added wildcard patterns
        };

        Ok((input, number))
    }

    fn string<'b>(&self, input: &'b str) -> IResult<&'b str, String> {
        use nom::bytes::complete::take_until;
        use nom::character::complete::char;
        use nom::sequence::delimited;

        delimited(char('"'), map(take_until("\""), String::from), char('"'))(input)
    }

    fn advance(&mut self, remaining: &'a str) {
        let consumed = self.input.len() - remaining.len();
        self.position += consumed;
        self.input = remaining;
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
