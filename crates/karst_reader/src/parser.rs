//! @path: karst/crates/karst_reader/parser.rs
//! @author: redskaber
//! @datetime: 2026-10-03
//! @discription: karst::crates::karst_reader::parser

use karst_span::{ByteOffset, ExpansionId, FileId, Span};
use karst_syntax::{Phase, ScopeSet, StxDatum, SyntaxObject};

use crate::{Delimiter, ReadError, Token, TokenKind};

pub const NESTING_LIMIT: usize = 256;

/// quick entry
pub fn parse_program(file_id: FileId, tokens: &[Token]) -> Result<Vec<SyntaxObject>, ReadError> {
    let mut parser = Parser::new(file_id, tokens);
    let mut froms: Vec<SyntaxObject> = Vec::new();
    loop {
        match parser.peek_kind() {
            None | Some(TokenKind::Eof) => return Ok(froms),
            Some(TokenKind::Delimiter(Delimiter::Close)) => {
                return Err(ReadError::UnexpectedClose {
                    at: parser.peek_offset(),
                });
            }
            Some(_) => froms.push(parser.parse_from()?),
        }
    }
}

struct Parser<'a> {
    file_id: FileId,
    tokens: &'a [Token],
    idx: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    pub fn new(file_id: FileId, tokens: &'a [Token]) -> Parser<'a> {
        Parser {
            file_id,
            tokens,
            idx: 0,
            depth: 0,
        }
    }
    fn peek_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.idx).map(|t| &t.kind)
    }
    fn peek_offset(&self) -> ByteOffset {
        self.tokens[self.idx].span.start
    }
    fn parse_from(&mut self) -> Result<SyntaxObject, ReadError> {
        let token = &self.tokens[self.idx];
        match &token.kind {
            // `(` `)`
            TokenKind::Delimiter(Delimiter::Open) => {
                let open_at = token.span.start;
                self.idx += 1;
                self.parse_list(open_at)
            }
            TokenKind::Delimiter(Delimiter::Close) => Err(ReadError::UnexpectedClose {
                at: token.span.start,
            }),
            // Identifier
            // TODO: handle
            TokenKind::Identifier(symbol) => {
                self.idx += 1;
                Ok(SyntaxObject::new(
                    StxDatum::Symbol(symbol.clone()),
                    token.span,
                    ScopeSet::new(),
                    Phase::Runtime,
                ))
            }
            TokenKind::IntLitral(v) => {
                self.idx += 1;
                Ok(SyntaxObject::int(
                    *v,
                    token.span,
                    ScopeSet::new(),
                    Phase::Runtime,
                ))
            }
            TokenKind::FloatLiteral(v) => {
                self.idx += 1;
                Ok(SyntaxObject::float(
                    *v,
                    token.span,
                    ScopeSet::new(),
                    Phase::Runtime,
                ))
            }
            TokenKind::BoolLiteral(v) => {
                self.idx += 1;
                Ok(SyntaxObject::bool(
                    *v,
                    token.span,
                    ScopeSet::new(),
                    Phase::Runtime,
                ))
            }
            // String
            TokenKind::StringLiteral(v) => {
                self.idx += 1;
                Ok(SyntaxObject::string(
                    v.clone(),
                    token.span,
                    ScopeSet::new(),
                    Phase::Runtime,
                ))
            }
            TokenKind::Eof => Err(ReadError::UnclosedList {
                open_at: token.span.start,
            }),
        }
    }

    fn parse_list(&mut self, open_at: ByteOffset) -> Result<SyntaxObject, ReadError> {
        self.depth += 1;
        if self.depth > NESTING_LIMIT {
            return Err(ReadError::NestingDepthExceeded {
                depth: self.depth,
                limit: NESTING_LIMIT,
            });
        }
        let mut items: Vec<SyntaxObject> = Vec::new();
        let close_span_end: ByteOffset;
        loop {
            match self.peek_kind() {
                None | Some(TokenKind::Eof) => {
                    return Err(ReadError::UnclosedList { open_at });
                }
                Some(TokenKind::Delimiter(Delimiter::Close)) => {
                    close_span_end = self.tokens[self.idx].span.end;
                    self.idx += 1;
                    break;
                }
                Some(_) => items.push(self.parse_from()?),
            }
        }
        self.depth -= 1;
        if items.is_empty() {
            return Err(ReadError::EmptyApplication { at: open_at });
        }
        let span = Span::new(self.file_id, open_at, close_span_end, ExpansionId::ROOT)
            .expect("list span is well-ordered by constrction");
        Ok(SyntaxObject::list(
            items,
            span,
            ScopeSet::new(),
            Phase::Runtime,
        ))
    }
}

#[cfg(test)]
mod tests {
    // more ...
}
