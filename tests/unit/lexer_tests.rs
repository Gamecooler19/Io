use crate::test_utils::{cleanup_test_env, setup_test_env};

#[derive(Debug, PartialEq)]
enum TestToken {
    Identifier(String),
    Number(i64),
    String(String),
    Symbol(char),
    Keyword(String),
    Error(String),
}

struct TestLexer {
    input: String,
    position: usize,
}

impl TestLexer {
    fn new(input: String) -> Self {
        Self { input, position: 0 }
    }

    fn next_token(&mut self) -> Option<TestToken> {
        self.skip_whitespace();
        if self.position >= self.input.len() {
            return None;
        }

        let c = self.current_char();
        match c {
            'a'..='z' | 'A'..='Z' | '_' => Some(self.read_identifier()),
            '0'..='9' => Some(self.read_number()),
            '"' => Some(self.read_string()),
            '+' | '-' | '*' | '/' | '=' => {
                self.position += 1;
                Some(TestToken::Symbol(c))
            }
            _ => Some(TestToken::Error(format!("Invalid character: {}", c))),
        }
    }

    fn current_char(&self) -> char {
        self.input.chars().nth(self.position).unwrap_or('\0')
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() && self.current_char().is_whitespace() {
            self.position += 1;
        }
    }

    fn read_identifier(&mut self) -> TestToken {
        let start = self.position;
        while self.position < self.input.len()
            && (self.current_char().is_alphanumeric() || self.current_char() == '_')
        {
            self.position += 1;
        }
        let identifier = &self.input[start..self.position];
        match identifier {
            "let" | "fn" | "if" | "else" | "return" => TestToken::Keyword(identifier.to_string()),
            _ => TestToken::Identifier(identifier.to_string()),
        }
    }

    fn read_number(&mut self) -> TestToken {
        let start = self.position;
        while self.position < self.input.len() && self.current_char().is_digit(10) {
            self.position += 1;
        }
        let number = self.input[start..self.position]
            .parse::<i64>()
            .expect("Failed to parse number");
        TestToken::Number(number)
    }

    fn read_string(&mut self) -> TestToken {
        self.position += 1; // Skip opening quote
        let start = self.position;
        while self.position < self.input.len() && self.current_char() != '"' {
            self.position += 1;
        }
        let string = self.input[start..self.position].to_string();
        self.position += 1; // Skip closing quote
        TestToken::String(string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_identifiers() {
        let mut lexer = TestLexer::new("abc xyz".to_string());
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Identifier("abc".to_string()))
        );
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Identifier("xyz".to_string()))
        );
        assert_eq!(lexer.next_token(), None);
    }

    #[test]
    fn test_lexer_numbers() {
        let mut lexer = TestLexer::new("123 456".to_string());
        assert_eq!(lexer.next_token(), Some(TestToken::Number(123)));
        assert_eq!(lexer.next_token(), Some(TestToken::Number(456)));
        assert_eq!(lexer.next_token(), None);
    }

    #[test]
    fn test_lexer_strings() {
        let mut lexer = TestLexer::new("\"hello\" \"world\"".to_string());
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::String("hello".to_string()))
        );
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::String("world".to_string()))
        );
        assert_eq!(lexer.next_token(), None);
    }

    #[test]
    fn test_lexer_keywords() {
        let mut lexer = TestLexer::new("let fn if else".to_string());
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Keyword("let".to_string()))
        );
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Keyword("fn".to_string()))
        );
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Keyword("if".to_string()))
        );
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Keyword("else".to_string()))
        );
        assert_eq!(lexer.next_token(), None);
    }

    #[test]
    fn test_lexer_symbols() {
        let mut lexer = TestLexer::new("+ - * /".to_string());
        assert_eq!(lexer.next_token(), Some(TestToken::Symbol('+')));
        assert_eq!(lexer.next_token(), Some(TestToken::Symbol('-')));
        assert_eq!(lexer.next_token(), Some(TestToken::Symbol('*')));
        assert_eq!(lexer.next_token(), Some(TestToken::Symbol('/')));
        assert_eq!(lexer.next_token(), None);
    }

    #[test]
    fn test_lexer_mixed_input() {
        let mut lexer = TestLexer::new("let x = 42 + \"test\"".to_string());
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Keyword("let".to_string()))
        );
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Identifier("x".to_string()))
        );
        assert_eq!(lexer.next_token(), Some(TestToken::Symbol('=')));
        assert_eq!(lexer.next_token(), Some(TestToken::Number(42)));
        assert_eq!(lexer.next_token(), Some(TestToken::Symbol('+')));
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::String("test".to_string()))
        );
        assert_eq!(lexer.next_token(), None);
    }

    #[test]
    fn test_lexer_error_handling() {
        let mut lexer = TestLexer::new("abc @ xyz".to_string());
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Identifier("abc".to_string()))
        );
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Error("Invalid character: @".to_string()))
        );
        assert_eq!(
            lexer.next_token(),
            Some(TestToken::Identifier("xyz".to_string()))
        );
        assert_eq!(lexer.next_token(), None);
    }
}
