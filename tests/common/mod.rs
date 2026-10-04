//! Shared helpers for cross-crate integration tests.

use karst::reader::{Delimiter, Token, TokenKind};
use karst::syntax::Symbol;
use karst_span::{ByteOffset, FileId, Span, SpanError};

pub(crate) fn open_token(span: Span) -> Token {
    Token {
        kind: TokenKind::Delimiter(Delimiter::Open),
        span,
    }
}

pub(crate) fn close_token(span: Span) -> Token {
    Token {
        kind: TokenKind::Delimiter(Delimiter::Close),
        span,
    }
}

pub(crate) fn ident_token(text: &str, span: Span) -> Token {
    Token {
        kind: TokenKind::Identifier(Symbol::new(text).expect("valid symbol")),
        span,
    }
}

pub(crate) fn int_token(value: i64, span: Span) -> Token {
    Token {
        kind: TokenKind::IntLiteral(value),
        span,
    }
}

pub(crate) fn covering_span(tokens: &[Token]) -> Result<Span, SpanError> {
    let mut span = tokens[0].span;
    for token in &tokens[1..] {
        span = span.join(&token.span)?;
    }
    Ok(span)
}

pub(crate) fn root_span(file: u32, start: u32, end: u32) -> Span {
    Span::root(FileId(file), ByteOffset(start), ByteOffset(end))
        .expect("test fixture span must be well-formed")
}
