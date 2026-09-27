//! @path: karst/crates/karst_reader/token.rs
//! @author: redskaber
//! @datetime: 2026-09-27
//! @discription: karst::crates::karst_reader::token

use std::rc::Rc;

use karst_span::Span;
use karst_syntax::{Keyword, ScopeSet, Symbol};

/// Operator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    Add, // +
    Sub, // -
    Nul, // *
    Div, // /
    Mod, // %
    Lt,  // >
    Gt,  // >=
    Le,  // <
    Ge,  // >=
    Eq,  // ==
}

/// Delimiter
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Delimiter {
    OpenParen,    // (
    CloseParen,   // )
    OpenBracket,  // [
    CloseBracket, // ]
}

impl Delimiter {
    pub fn as_char(self) -> char {
        match self {
            Delimiter::OpenParen => '(',
            Delimiter::CloseParen => ')',
            Delimiter::OpenBracket => '[',
            Delimiter::CloseBracket => ']',
        }
    }

    pub fn closing_of(open: Delimiter) -> Delimiter {
        match open {
            Delimiter::OpenParen => Delimiter::CloseParen,
            Delimiter::OpenBracket => Delimiter::CloseBracket,
            _ => Delimiter::CloseParen, // caller handle
        }
    }
}

/// token kind
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// literal
    IntLitral(i64),
    FloatLiteral(f64),
    StringLiteral(Rc<str>),
    BoolLiteral(bool),
    NilLiteral,

    /// ident
    Identifier(Symbol),
    QuoteShorthand, // 'x => (quote x)
    Keyword(Keyword),
    Operator(Operator, Symbol), // sign
    Delimiter(Delimiter),

    /// interface macro extra
    MacroInvocation(Symbol),
    Eof,
}

/// token
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// token kind
    pub kind: TokenKind,
    /// token position
    pub span: Span,
    /// token scopes
    pub scopes: ScopeSet,
}

impl Token {
    pub fn is_eof(&self) -> bool {
        matches!(self.kind, TokenKind::Eof)
    }

    pub fn is_keyword(&self, kw: Keyword) -> bool {
        matches!(&self.kind, TokenKind::Keyword(k) if *k == kw)
    }
}

/// reader error handle
#[derive(Debug, Clone, PartialEq)]
pub struct ReadError {
    pub message: String,
    pub span: Span,
}

impl ReadError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        ReadError {
            message: message.into(),
            span,
        }
    }

    pub fn unexpected_token(token: &Token) -> Self {
        ReadError::new(format!("unexpected token: {:?}", token.kind), token.span)
    }

    pub fn unclosed_delimiter(open: Delimiter, span: Span) -> Self {
        ReadError::new(
            format!(
                "unclosing delimiter: '{}'",
                Delimiter::closing_of(open).as_char()
            ),
            span,
        )
    }

    pub fn stray_closing_delimiter(close: Delimiter, span: Span) -> Self {
        ReadError::new(
            format!("stray closing delimiter: '{}'", close.as_char()),
            span,
        )
    }
}
