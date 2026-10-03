//! @path: karst/crates/karst_syntax/error.rs
//! @author: redskaber
//! @datetime: 2026-09-30
//! @discription: karst::crates::karst_syntax::error
//!
//! Errors produced by karst_syntax  construction and validation

use std::error::Error;
use std::fmt;

/// karst_syntax error set
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyntaxError {
    /// a name failed [`crate::Symbol`](crate::symbol::Symbol) floor.
    InvalidSymbol { text: String },
    /// `Phase::from_u8` received a level outside the frozen two level.
    InvalidPhase { phase: u8 },
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyntaxError::InvalidSymbol { text } => {
                write!(
                    f,
                    "invalid symbol `{text}`: must be non-empty with no withespace or control charecters"
                )
            }
            SyntaxError::InvalidPhase { phase } => {
                write!(
                    f,
                    "invalid phase: `{phase}`: must be 0 (runtime) or 1 (expandtime)"
                )
            }
        }
    }
}

impl Error for SyntaxError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_invalid_symbol_mentions_text_and_rules() {
        let err = SyntaxError::InvalidSymbol {
            text: "a b".to_owned(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("`a b`"));
        assert!(rendered.contains("non-empty"));
        assert!(rendered.contains("withespace"));
    }

    // more ...
}
