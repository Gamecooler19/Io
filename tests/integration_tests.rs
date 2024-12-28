use inkwell::context::Context;
use io_lang::{
    codegen::CodeGenerator, error::IoError, lexer::Lexer, optimizer::Optimizer, parser::Parser,
    semantic::SemanticAnalyzer,
};

#[test]
fn test_full_compilation_pipeline() -> Result<(), IoError> {
    let source = r#"
        fn main() {
            println("Hello, World!");
        }
    "#;

    let context = Context::create();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;

    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;

    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;

    let mut codegen = CodeGenerator::new(&context, "test_module");
    codegen.generate(&ast)?;

    let optimizer = Optimizer::new(&codegen.module);
    optimizer.optimize_module(&codegen.module)?;

    codegen.verify_module()?;

    Ok(())
}
