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
                let opened_at = token.span.start;
                self.idx += 1;
                self.parse_list(opened_at)
            }
            TokenKind::Delimiter(Delimiter::Close) => Err(ReadError::UnexpectedClose {
                at: token.span.start,
            }),
            // Identifier
            TokenKind::Identifier(symbol) => {
                self.idx += 1;
                Ok(SyntaxObject::new(
                    StxDatum::Symbol(symbol.clone()),
                    token.span,
                    ScopeSet::new(),
                    Phase::Runtime,
                ))
            }
            TokenKind::IntLiteral(v) => {
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
                opened_at: token.span.start,
            }),
        }
    }

    fn parse_list(&mut self, opened_at: ByteOffset) -> Result<SyntaxObject, ReadError> {
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
                    // The innermost open list — the one being parsed
                    // when input ran out — reports its own `(` first as
                    // the error propagates up: deterministic boundary,
                    // and the closest location to where input ended.
                    return Err(ReadError::UnclosedList { opened_at });
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
            return Err(ReadError::EmptyApplication { at: opened_at });
        }
        let span = Span::new(self.file_id, opened_at, close_span_end, ExpansionId::ROOT)
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
    //! Unit tests for the parser feature point (sub-stage test doc
    //! FP2/FP4).

    use super::*;

    fn read(src: &str) -> Result<Vec<SyntaxObject>, ReadError> {
        crate::read_program(FileId(0), src)
    }

    fn deep_source(depth: usize) -> String {
        let mut src = String::new();
        for _ in 0..depth {
            src.push('(');
        }
        src.push('x');
        for _ in 0..depth {
            src.push(')');
        }
        src
    }

    // ---------- positive ----------

    #[test]
    fn parses_multi_form_program_with_stamps() {
        let forms = read("(a) 1 x").expect("three top-level forms");
        assert_eq!(forms.len(), 3);
        assert_eq!(forms[0].datum.to_string(), "(a)");
        assert_eq!(forms[1].datum.to_string(), "1");
        assert_eq!(forms[2].datum.to_string(), "x");
        for form in &forms {
            assert_eq!(form.phase, Phase::Runtime);
            assert!(form.scopes.is_empty());
        }
    }

    #[test]
    fn nested_tree_carries_exact_byte_spans() {
        let forms = read("(f (g 1) 2)").expect("nested program parses");
        let root = &forms[0];
        assert_eq!((root.span.start.0, root.span.end.0), (0, 11));
        let mut order = Vec::new();
        root.visit_spans(&mut |s| order.push((s.start.0, s.end.0)));
        // Root, `f`, inner list, `g`, `1`, `2` — pre-order.
        assert_eq!(
            order,
            vec![(0, 11), (1, 2), (3, 8), (4, 5), (6, 7), (9, 10)]
        );
    }

    #[test]
    fn whitespace_and_comment_only_source_is_empty_program() {
        let forms = read(" ; nothing here\n\t\n").expect("trivia-only source reads");
        assert!(forms.is_empty());
    }

    #[test]
    fn nesting_limit_boundary_is_inclusive() {
        let forms = read(&deep_source(NESTING_LIMIT)).expect("256-deep nesting is legal");
        let mut node = &forms[0];
        let mut depth = 1usize;
        while let StxDatum::List(items) = &node.datum {
            if items.is_empty() {
                break;
            }
            node = &items[0];
            depth += 1;
        }
        assert_eq!(depth, NESTING_LIMIT + 1, "root + 256 nested lists");
    }

    // ---------- negative ----------

    #[test]
    fn rejects_unclosed_list() {
        let err = read("(f").expect_err("unclosed list must fail");
        assert_eq!(
            err,
            ReadError::UnclosedList {
                opened_at: ByteOffset(0)
            }
        );
    }

    #[test]
    fn rejects_unclosed_inner_list() {
        let err = read("(a (b").expect_err("unclosed inner list must fail");
        assert_eq!(
            err,
            ReadError::UnclosedList {
                opened_at: ByteOffset(3)
            }
        );
    }

    #[test]
    fn rejects_stray_close() {
        let err = read(")").expect_err("stray close must fail");
        assert_eq!(err, ReadError::UnexpectedClose { at: ByteOffset(0) });
    }

    #[test]
    fn rejects_stray_close_after_forms() {
        let err = read("(a 1) )").expect_err("stray close after forms must fail");
        assert_eq!(err, ReadError::UnexpectedClose { at: ByteOffset(6) });
    }

    #[test]
    fn rejects_empty_application() {
        let err = read("()").expect_err("empty application must fail");
        assert_eq!(err, ReadError::EmptyApplication { at: ByteOffset(0) });
    }

    #[test]
    fn rejects_nested_empty_application() {
        let err = read("(f ())").expect_err("nested empty application must fail");
        assert_eq!(err, ReadError::EmptyApplication { at: ByteOffset(3) });
    }

    #[test]
    fn rejects_empty_application_deep_inside() {
        let err = read("(a (b ()))").expect_err("deep empty application must fail");
        assert_eq!(err, ReadError::EmptyApplication { at: ByteOffset(6) });
    }

    #[test]
    fn rejects_empty_application_in_argument_position() {
        let err = read("(f () 1)").expect_err("empty argument must fail");
        assert_eq!(err, ReadError::EmptyApplication { at: ByteOffset(3) });
    }

    #[test]
    fn rejects_unclosed_locates_third_form() {
        let err = read("(a) (b) (c").expect_err("unclosed third form must fail");
        assert_eq!(
            err,
            ReadError::UnclosedList {
                opened_at: ByteOffset(8)
            }
        );
    }

    #[test]
    fn rejects_nesting_over_the_limit() {
        let err = read(&deep_source(NESTING_LIMIT + 1)).expect_err("257-deep nesting must fail");
        assert_eq!(
            err,
            ReadError::NestingDepthExceeded {
                depth: NESTING_LIMIT + 1,
                limit: NESTING_LIMIT,
            }
        );
    }

    #[test]
    fn unclosed_error_locates_the_form() {
        let err = read("(a 1) (b (c) x").expect_err("mid-program error must locate");
        assert_eq!(
            err,
            ReadError::UnclosedList {
                opened_at: ByteOffset(6)
            }
        );
    }

    #[test]
    fn multiple_open_lists_report_innermost() {
        let err = read("(((x").expect_err("stacked opens must fail");
        assert_eq!(
            err,
            ReadError::UnclosedList {
                opened_at: ByteOffset(2)
            }
        );
    }

    #[test]
    fn empty_application_fires_before_outer_unclosed() {
        let err = read("(()").expect_err("inner empty application must fail");
        assert_eq!(err, ReadError::EmptyApplication { at: ByteOffset(1) });
    }
}
