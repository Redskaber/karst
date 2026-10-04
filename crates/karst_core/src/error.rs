//! @path: karst/crates/karst_core/error.rs
//! @author: redskaber
//! @datetime: 2026-10-01
//! @discription: karst::crates::karst_core::error
//!
//! the frozen semantic primitive kernel error

use std::error::Error;
use std::fmt;

use karst_syntax::{Symbol, SyntaxError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// A name failed the [`crate::Symbol`](crate::symbol::Symbol) floor
    /// (empty, whitespace, or control character).
    InvalidSymbol { text: String },
    /// `fn` form binds the same parameter name twice.
    DuplicateParam { name: String },
    /// `handle` installs two clauses for the same effect.
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
    //! Unit tests for the CoreError rendering surface (sub-stage test doc
    //! FP4; R8 message-shape assertions).

    use super::*;

    // ---------- negative (error surface) ----------

    #[test]
    fn display_invalid_symbol_mentions_text_and_rules() {
        let err = CoreError::InvalidSymbol {
            text: "a b".to_owned(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("`a b`"));
        assert!(rendered.contains("non-empty"));
        assert!(rendered.contains("whitespace"));
        assert!(!rendered.ends_with('.'));
    }

    #[test]
    fn display_duplicate_param_mentions_name() {
        let err = CoreError::DuplicateParam {
            name: "x".to_owned(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("`x`"));
        assert!(rendered.contains("fn"));
        assert_eq!(rendered.chars().next().unwrap(), 'd');
    }

    #[test]
    fn display_duplicate_handler_mentions_effect() {
        let err = CoreError::DuplicateHandler {
            effect: "read".to_owned(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("`read`"));
        assert!(rendered.contains("handler"));
    }

    #[test]
    fn error_variants_distinguishable_by_equality() {
        let a = CoreError::InvalidSymbol {
            text: "x".to_owned(),
        };
        let b = CoreError::DuplicateParam {
            name: "x".to_owned(),
        };
        let c = CoreError::DuplicateHandler {
            effect: "x".to_owned(),
        };
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
        // Same variant with same payload compares equal (test assertions
        // rely on this).
        assert_eq!(
            a,
            CoreError::InvalidSymbol {
                text: "x".to_owned()
            }
        );
    }
}
