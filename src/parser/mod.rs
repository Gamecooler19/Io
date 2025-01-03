use crate::{
    ast::{ASTNode, BinaryOperator, Parameter},
    token::{Token, TokenKind},
    Result,
};
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

    pub fn parse_program(&mut self) -> Result<Vec<ASTNode>> {
        let mut nodes = Vec::new();
        while self.current.is_some() {
            nodes.push(self.parse_declaration()?);
        }
        Ok(nodes)
    }

    fn parse_declaration(&mut self) -> Result<ASTNode> {
        match &self.current {
            Some(Token {
                kind: TokenKind::Function,
                ..
            }) => self.parse_function(),
            Some(Token {
                kind: TokenKind::Let,
                ..
            }) => self.parse_variable_declaration(),
            Some(Token {
                kind: TokenKind::Type,
                ..
            }) => self.parse_type_declaration(),
            _ => self.parse_statement(),
        }
    }

    fn parse_function(&mut self) -> Result<ASTNode> {
        self.advance(); // consume 'fn'
        let name = self.expect_token(TokenKind::Identifier)?.value;

        self.expect_token(TokenKind::LeftParen)?;
        let parameters = self.parse_parameters()?;
        self.expect_token(TokenKind::RightParen)?;

        let return_type = if self.match_token(&[TokenKind::Arrow]) {
            Some(self.expect_token(TokenKind::Identifier)?.value)
        } else {
            None
        };

        self.expect_token(TokenKind::LeftBrace)?;
        let body = self.parse_block()?;
        self.expect_token(TokenKind::RightBrace)?;

        Ok(ASTNode::Function {
            name,
            parameters,
            return_type,
            body,
            is_async: false,
        })
    }

    fn parse_parameters(&mut self) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();

        if !self.match_token(&[TokenKind::RightParen]) {
            loop {
                let name = self.expect_token(TokenKind::Identifier)?.value;
                self.expect_token(TokenKind::Colon)?;
                let type_name = self.expect_token(TokenKind::Identifier)?.value;

                parameters.push(Parameter {
                    name,
                    type_annotation: type_name,
                });

                if !self.match_token(&[TokenKind::Comma]) {
                    break;
                }
            }
        }

        Ok(parameters)
    }

    fn parse_block(&mut self) -> Result<Vec<ASTNode>> {
        let mut statements = Vec::new();
        while !self.match_token(&[TokenKind::RightBrace]) {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<ASTNode> {
        match &self.current {
            Some(Token {
                kind: TokenKind::If,
                ..
            }) => self.parse_if_statement(),
            Some(Token {
                kind: TokenKind::While,
                ..
            }) => self.parse_while_statement(),
            Some(Token {
                kind: TokenKind::For,
                ..
            }) => self.parse_for_statement(),
            Some(Token {
                kind: TokenKind::Return,
                ..
            }) => self.parse_return_statement(),
            Some(Token {
                kind: TokenKind::Break,
                ..
            }) => {
                self.advance();
                Ok(ASTNode::Break)
            }
            Some(Token {
                kind: TokenKind::Continue,
                ..
            }) => {
                self.advance();
                Ok(ASTNode::Continue)
            }
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_if_statement(&mut self) -> Result<ASTNode> {
        self.advance(); // consume 'if'
        let condition = Box::new(self.parse_expression()?);

        self.expect_token(TokenKind::LeftBrace)?;
        let then_branch = self.parse_block()?;
        self.expect_token(TokenKind::RightBrace)?;

        let else_branch = if self.match_token(&[TokenKind::Else]) {
            self.expect_token(TokenKind::LeftBrace)?;
            let branch = self.parse_block()?;
            self.expect_token(TokenKind::RightBrace)?;
            Some(branch)
        } else {
            None
        };

        Ok(ASTNode::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_expression(&mut self) -> Result<ASTNode> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<ASTNode> {
        let expr = self.parse_logical_or()?;

        if self.match_token(&[TokenKind::Equal]) {
            let value = Box::new(self.parse_assignment()?);
            match expr {
                ASTNode::Identifier(name) => Ok(ASTNode::Assignment {
                    target: Box::new(ASTNode::Identifier(name)),
                    value,
                }),
                _ => Err("Invalid assignment target".into()),
            }
        } else {
            Ok(expr)
        }
    }

    fn parse_logical_or(&mut self) -> Result<ASTNode> {
        let mut expr = self.parse_logical_and()?;

        while self.match_token(&[TokenKind::Or]) {
            let operator = BinaryOperator::Or;
            let right = self.parse_logical_and()?;
            expr = ASTNode::BinaryOp {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<ASTNode> {
        let mut expr = self.parse_equality()?;

        while self.match_token(&[TokenKind::And]) {
            let operator = BinaryOperator::And;
            let right = self.parse_equality()?;
            expr = ASTNode::BinaryOp {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<ASTNode> {
        let mut expr = self.parse_comparison()?;

        while self.match_token(&[TokenKind::EqualEqual, TokenKind::BangEqual]) {
            let operator = match self.current.as_ref().unwrap().kind {
                TokenKind::EqualEqual => BinaryOperator::Equal,
                TokenKind::BangEqual => BinaryOperator::NotEqual,
                _ => unreachable!(),
            };
            let right = self.parse_comparison()?;
            expr = ASTNode::BinaryOp {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<ASTNode> {
        match &self.current {
            Some(Token {
                kind: TokenKind::Number,
                value,
            }) => {
                self.advance();
                Ok(ASTNode::NumberLiteral(value.parse()?))
            }
            Some(Token {
                kind: TokenKind::String,
                value,
            }) => {
                self.advance();
                Ok(ASTNode::StringLiteral(value.clone()))
            }
            Some(Token {
                kind: TokenKind::True,
            }) => {
                self.advance();
                Ok(ASTNode::BooleanLiteral(true))
            }
            Some(Token {
                kind: TokenKind::False,
            }) => {
                self.advance();
                Ok(ASTNode::BooleanLiteral(false))
            }
            Some(Token {
                kind: TokenKind::Identifier,
                value,
            }) => {
                self.advance();
                Ok(ASTNode::Identifier(value.clone()))
            }
            Some(Token {
                kind: TokenKind::LeftParen,
                ..
            }) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_token(TokenKind::RightParen)?;
                Ok(expr)
            }
            Some(Token {
                kind: TokenKind::LeftBracket,
                ..
            }) => self.parse_array_literal(),
            _ => Err("Unexpected token in primary expression".into()),
        }
    }

    fn parse_array_literal(&mut self) -> Result<ASTNode> {
        self.advance(); // consume '['
        let mut elements = Vec::new();

        if !self.match_token(&[TokenKind::RightBracket]) {
            loop {
                elements.push(self.parse_expression()?);
                if !self.match_token(&[TokenKind::Comma]) {
                    break;
                }
            }
            self.expect_token(TokenKind::RightBracket)?;
        }

        Ok(ASTNode::ArrayLiteral(elements))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_function() {
        let tokens = vec![
            Token::new(TokenKind::Function, "fn"),
            Token::new(TokenKind::Identifier, "test"),
            Token::new(TokenKind::LeftParen, "("),
            Token::new(TokenKind::RightParen, ")"),
            Token::new(TokenKind::LeftBrace, "{"),
            Token::new(TokenKind::RightBrace, "}"),
        ];

        let mut parser = Parser::new(tokens.into_iter());
        let result = parser.parse_function();
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_if_statement() {
        let tokens = vec![
            Token::new(TokenKind::If, "if"),
            Token::new(TokenKind::True, "true"),
            Token::new(TokenKind::LeftBrace, "{"),
            Token::new(TokenKind::RightBrace, "}"),
        ];

        let mut parser = Parser::new(tokens.into_iter());
        let result = parser.parse_if_statement();
        assert!(result.is_ok());
    }
}
