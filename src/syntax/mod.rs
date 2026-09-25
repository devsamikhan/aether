pub mod ast;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

pub use ast::*;
pub use lexer::Lexer;
pub use parser::Parser;
pub use span::Span;
pub use token::{Token, TokenKind};

/// Helper function to parse an AETHER source string directly into a Program AST.
pub fn parse(source: &str) -> Result<Program, (String, Span)> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}
