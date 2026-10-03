//! @path: karst/crates/karst_reader/lib.rs
//! @author: redskaber
//! @datetime: 2026-09-27
//! @discription: karst::crates::karst_reader (lexer + parser)

pub mod error;
pub mod lexer;
pub mod parser;
pub mod token;

pub use error::ReadError;
pub use lexer::lex;
pub use parser::{NESTING_LIMIT, parse_program};
pub use token::{Delimiter, Token, TokenKind};

/// Top-level froms
pub fn read_program(
    file_id: karst_span::FileId,
    source: &str,
) -> Result<Vec<karst_syntax::SyntaxObject>, ReadError> {
    let tokens = lex(file_id, source)?;
    parse_program(file_id, &tokens)
}
