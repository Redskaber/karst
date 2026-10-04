//! @path: karst/crates/karst_syntax/symbol.rs
//! @author: redskaber
//! @datetime: 2026-09-26
//! @discription: karst::crates::karst_syntax::symbol
//!
//! Surface-syntax ident symbol: validated, opaque, cheap to compare
//! start simple -> full

use std::fmt;

use crate::error::SyntaxError;

/// simple symbol design and impl
/// TODO: interner symbol, nfc
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol {
    text: String,
}

impl Symbol {
    pub fn new(text: &str) -> Result<Symbol, SyntaxError> {
        if text.is_empty() {
            return Err(SyntaxError::InvalidSymbol {
                text: text.to_owned(),
            });
        }
        if text.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(SyntaxError::InvalidSymbol {
                text: text.to_owned(),
            });
        }
        Ok(Symbol {
            text: text.to_owned(),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- positive ----------

    #[test]
    fn valid_symbols_roundtrip() {
        for text in [
            "x",
            "snake_case_x1",
            "+",
            "x!",
            "?value",
            "a1b2",
            "nil?",
            "set!",
        ] {
            let symbol = Symbol::new(text).expect("floor-valid name must construct");
            assert_eq!(symbol.as_str(), text);
            assert_eq!(symbol.to_string(), text);
        }
        let a = Symbol::new("alpha").unwrap();
        let b = Symbol::new("alpha").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, Symbol::new("beta").unwrap());
    }

    #[test]
    fn valid_symbols_accept_unicode_cjk() {
        // The floor must not constrain the future reader grammar: CJK
        // identifiers stay legal until the 0.3 grammar rules on them.
        let symbol = Symbol::new("计数器").expect("CJK identifier passes the floor");
        assert_eq!(symbol.as_str(), "计数器");
    }

    // ---------- negative ----------

    #[test]
    fn new_rejects_empty() {
        let err = Symbol::new("").expect_err("empty symbol must be rejected");
        assert_eq!(
            err,
            SyntaxError::InvalidSymbol {
                text: String::new()
            }
        );
    }

    #[test]
    fn new_rejects_space_only() {
        assert!(matches!(
            Symbol::new(" "),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn new_rejects_interior_space() {
        assert!(matches!(
            Symbol::new("a b"),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn new_rejects_interior_tab() {
        assert!(matches!(
            Symbol::new("a\tb"),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn new_rejects_leading_space() {
        assert!(matches!(
            Symbol::new(" x"),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn new_rejects_trailing_space() {
        assert!(matches!(
            Symbol::new("x "),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn new_rejects_nul_control() {
        assert!(matches!(
            Symbol::new("a\u{0}b"),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn new_rejects_unicode_whitespace() {
        // Unicode whitespace (NBSP) is whitespace for the floor too —
        // `char::is_whitespace` is Unicode-aware.
        assert!(matches!(
            Symbol::new("a\u{a0}b"),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }
}
