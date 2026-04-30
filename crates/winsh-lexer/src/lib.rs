//! # WinSH Lexer
//!
//! Tokenizer/lexer for the WinSH shell language.
//! Converts raw input text into a stream of tokens.

pub mod token;
pub mod lexer;
pub mod quote;

pub use lexer::Lexer;
pub use token::Token;
pub use winsh_ast::token::TokenKind;
