//! @path: karst/crates/karst_reader/token.rs
//! @author: redskaber
//! @datetime: 2026-10-02
//! @discription: karst::crates::karst_reader::token
//!
//! the typed token stream of the S-experssion skin

use core::fmt;

use karst_span::Span;
use karst_syntax::Symbol;

/// Delimiter - S-experssion `(`, `)`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delimiter {
    Open,  // (
    Close, // )
}

impl Delimiter {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Delimiter::Open => "open",
            Delimiter::Close => "close",
        }
    }
}

impl fmt::Display for Delimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Delimiter::Open => "(",
            Delimiter::Close => ")",
        })
    }
}

/// token kind
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Delimiter(Delimiter),
    Identifier(Symbol),

    /// literal
    IntLitral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),

    Eof,
}

impl TokenKind {
    pub fn kind_name(&self) -> &'static str {
        match self {
            TokenKind::Delimiter(_) => "delimiter",
            TokenKind::Identifier(_) => "ident",
            TokenKind::IntLitral(_) => "int",
            TokenKind::FloatLiteral(_) => "float",
            TokenKind::StringLiteral(_) => "string",
            TokenKind::BoolLiteral(_) => "bool",
            TokenKind::Eof => "eof",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Delimiter(d) => write!(f, "{d}"),
            TokenKind::Identifier(s) => f.write_str(s.as_str()),
            TokenKind::IntLitral(v) => write!(f, "{v}"),
            TokenKind::FloatLiteral(v) => write!(f, "{v:?}"),
            TokenKind::StringLiteral(v) => write!(f, "{v:?}"),
            TokenKind::BoolLiteral(v) => write!(f, "#{}", if *v { "t" } else { "f" }),
            TokenKind::Eof => f.write_str("end of input"),
        }
    }
}

/// token
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// token kind
    pub kind: TokenKind,
    /// token position
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Token {
        Token { kind, span }
    }
    pub fn kind_name(&self) -> &'static str {
        self.kind.kind_name()
    }
}

#[cfg(test)]
mod tests {
    // more ...
}
