//! @path: karst/crates/karst_syntax/symbol.rs
//! @author: redskaber
//! @datetime: 2026-09-26
//! @discription: karst::crates::karst_syntax::symbol
//!
//! Surface-syntax ident symbol: validated, opaque, cheap to compare
//! start simple -> full

use core::fmt;

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
        assert_eq!(a, Symbol::new("beta").unwrap())
    }

    // more ...
}
