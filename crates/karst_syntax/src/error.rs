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
                    "invalid symbol `{text}`: must be non-empty with no whitespace or control characters"
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
    //! Unit tests for the SyntaxError rendering surface (sub-stage test
    //! doc FP1/FP2; R8 message-shape assertions).

    use super::*;

    // ---------- negative (error surface) ----------

    #[test]
    fn display_invalid_symbol_mentions_text_and_rules() {
        let err = SyntaxError::InvalidSymbol {
            text: "a b".to_owned(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("`a b`"));
        assert!(rendered.contains("non-empty"));
        assert!(rendered.contains("whitespace"));
        assert!(!rendered.ends_with('.'));
    }

    #[test]
    fn display_invalid_phase_mentions_level_and_valid_values() {
        let err = SyntaxError::InvalidPhase { phase: 2 };
        let rendered = err.to_string();
        assert!(rendered.contains("`2`"));
        assert!(rendered.contains("runtime"));
        assert!(rendered.contains("expandtime"));
        assert!(!rendered.ends_with('.'));
    }

    #[test]
    fn error_variants_distinguishable_by_equality() {
        let a = SyntaxError::InvalidSymbol {
            text: "x".to_owned(),
        };
        let b = SyntaxError::InvalidPhase { phase: 9 };
        assert_ne!(a, b);
        // Same variant with same payload compares equal (test assertions
        // rely on this).
        assert_eq!(
            a,
            SyntaxError::InvalidSymbol {
                text: "x".to_owned()
            }
        );
    }
}
