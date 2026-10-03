//! @path: karst/crates/karst_core/error.rs
//! @author: redskaber
//! @datetime: 2026-10-01
//! @discription: karst::crates::karst_core::error
//!
//! the frozen semantic primitive kernel error

use core::fmt;
use std::error::Error;

use karst_syntax::{Symbol, SyntaxError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidSymbol { text: String },
    DuplicateParam { name: String },
    DuplicateHandler { effect: String },
}

pub fn from_syntax_symbol(raw: Result<Symbol, SyntaxError>) -> Result<Symbol, CoreError> {
    match raw {
        Ok(s) => Ok(s),
        Err(e) => match e {
            SyntaxError::InvalidSymbol { text } => Err(CoreError::InvalidSymbol { text }),
            _ => unreachable!(),
        },
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::InvalidSymbol { text } => (SyntaxError::InvalidSymbol {
                text: text.as_str().to_owned(),
            })
            .fmt(f),
            CoreError::DuplicateParam { name } => {
                write!(f, "duplicate parameter `{name}` in fn form")
            }
            CoreError::DuplicateHandler { effect } => {
                write!(f, "duplicate handler clause for effect `{effect}`")
            }
        }
    }
}

impl Error for CoreError {}

#[cfg(test)]
mod tests {

    // more ..
}
