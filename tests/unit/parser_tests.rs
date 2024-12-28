use crate::test_utils::setup_test_env;

#[derive(Debug, PartialEq)]
enum AstNode {
    Program(Vec<AstNode>),
    Statement(Box<AstNode>),
    Expression(Box<AstNode>),
    BinaryOp {
        left: Box<AstNode>,
        operator: String,
        right: Box<AstNode>,
    },
    Variable(String),
    Number(i64),
    StringLiteral(String),
    FunctionCall {
        name: String,
        arguments: Vec<AstNode>,
    },
}

struct Parser {
    tokens: Vec<TestToken>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<TestToken>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn parse_program(&mut self) -> Result<AstNode, String> {
        let mut statements = Vec::new();
        while self.position < self.tokens.len() {
            statements.push(self.parse_statement()?);
        }
        Ok(AstNode::Program(statements))
    }

    fn parse_statement(&mut self) -> Result<AstNode, String> {
        match self.current_token() {
            Some(TestToken::Keyword(ref s)) if s == "let" => self.parse_let_statement(),
            Some(_) => self.parse_expression_statement(),
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn parse_expression_statement(&mut self) -> Result<AstNode, String> {
        let expr = self.parse_expression(0)?;
        Ok(AstNode::Statement(Box::new(expr)))
    }

    fn parse_expression(&mut self, _precedence: i32) -> Result<AstNode, String> {
        match self.current_token() {
            Some(TestToken::Number(n)) => {
                self.position += 1;
                Ok(AstNode::Number(n))
            }
            Some(TestToken::String(s)) => {
                self.position += 1;
                Ok(AstNode::StringLiteral(s.clone()))
            }
            Some(TestToken::Identifier(s)) => {
                self.position += 1;
                if self.peek_token() == Some(&TestToken::Symbol('(')) {
                    self.parse_function_call(s)
                } else {
                    Ok(AstNode::Variable(s.clone()))
                }
            }
            _ => Err("Unexpected token".to_string()),
        }
    }

    fn parse_function_call(&mut self, name: String) -> Result<AstNode, String> {
        self.position += 1; // consume '('
        let mut args = Vec::new();
        while self.current_token() != Some(TestToken::Symbol(')')) {
            args.push(self.parse_expression(0)?);
            if self.current_token() == Some(TestToken::Symbol(',')) {
                self.position += 1;
            }
        }
        self.position += 1; // consume ')'
        Ok(AstNode::FunctionCall {
            name,
            arguments: args,
        })
    }

    fn current_token(&self) -> Option<&TestToken> {
        self.tokens.get(self.position)
    }

    fn peek_token(&self) -> Option<&TestToken> {
        self.tokens.get(self.position + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unit::lexer_tests::TestToken;

    #[test]
    fn test_parse_number() {
        let tokens = vec![TestToken::Number(42)];
        let mut parser = Parser::new(tokens);
        let result = parser.parse_program().unwrap();
        assert_eq!(
            result,
            AstNode::Program(vec![AstNode::Statement(Box::new(AstNode::Number(42)))])
        );
    }

    #[test]
    fn test_parse_string() {
        let tokens = vec![TestToken::String("hello".to_string())];
        let mut parser = Parser::new(tokens);
        let result = parser.parse_program().unwrap();
        assert_eq!(
            result,
            AstNode::Program(vec![AstNode::Statement(Box::new(AstNode::StringLiteral(
                "hello".to_string()
            )))])
        );
    }

    #[test]
    fn test_parse_variable() {
        let tokens = vec![TestToken::Identifier("x".to_string())];
        let mut parser = Parser::new(tokens);
        let result = parser.parse_program().unwrap();
        assert_eq!(
            result,
            AstNode::Program(vec![AstNode::Statement(Box::new(AstNode::Variable(
                "x".to_string()
            )))])
        );
    }

    #[test]
    fn test_parse_function_call() {
        let tokens = vec![
            TestToken::Identifier("print".to_string()),
            TestToken::Symbol('('),
            TestToken::String("hello".to_string()),
            TestToken::Symbol(')'),
        ];
        let mut parser = Parser::new(tokens);
        let result = parser.parse_program().unwrap();
        assert_eq!(
            result,
            AstNode::Program(vec![AstNode::Statement(Box::new(AstNode::FunctionCall {
                name: "print".to_string(),
                arguments: vec![AstNode::StringLiteral("hello".to_string())],
            }))])
        );
    }

    #[test]
    fn test_parse_error_handling() {
        let tokens = vec![TestToken::Symbol('+')];
        let mut parser = Parser::new(tokens);
        assert!(parser.parse_program().is_err());
    }
}
