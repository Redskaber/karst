//! @path: karst/crates/karst_reader/lexer.rs
//! @author: redskaber
//! @datetime: 2026-10-02
//! @discription: karst::crates::karst_reader::lexer
//!
//! source text -> typed token stram

use crate::{Delimiter, Token, TokenKind, error::ReadError};

use karst_span::{ByteOffset, ExpansionId, FileId, Span};
use karst_syntax::Symbol;

// quick entry
pub fn lex(file_id: FileId, source: &str) -> Result<Vec<Token>, ReadError> {
    let mut lexer = Lexer::new(file_id, source);
    let mut tokens = Vec::new();
    loop {
        lexer.skip_trivial()?;
        if lexer.at_end() {
            tokens.push(Token::new(
                TokenKind::Eof,
                span(file_id, lexer.pos, lexer.pos),
            ));
            return Ok(tokens);
        }
        tokens.push(lexer.next_token()?);
    }
}

struct Lexer<'a> {
    /// file id
    file_id: FileId,
    /// source text
    source: &'a str,
    /// current byte position
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(file_id: FileId, source: &'a str) -> Lexer<'a> {
        Lexer {
            file_id,
            source,
            pos: 0,
        }
    }

    pub fn at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek_char(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    // move
    fn advance(&mut self, ch: char) {
        self.pos += ch.len_utf8();
    }

    fn skip_trivial(&mut self) -> Result<(), ReadError> {
        loop {
            let Some(ch) = self.peek_char() else {
                return Ok(());
            };
            if ch.is_whitespace() || ch == '\u{feff}' {
                self.advance(ch);
            } else if ch == ';' {
                // comment
                while let Some(c) = self.peek_char() {
                    if c == '\n' {
                        break;
                    }
                    self.advance(c);
                }
            } else {
                return Ok(());
            }
        }
    }

    // nested entry
    fn next_token(&mut self) -> Result<Token, ReadError> {
        let start = self.pos;
        let first = self.peek_char().expect("caller checked non-end");
        match first {
            // open
            '(' => {
                self.advance('(');
                Ok(Token::new(
                    TokenKind::Delimiter(Delimiter::Open),
                    span(self.file_id, start, self.pos),
                ))
            }
            // close
            ')' => {
                self.advance(')');
                Ok(Token::new(
                    TokenKind::Delimiter(Delimiter::Close),
                    span(self.file_id, start, self.pos),
                ))
            }
            '"' => self.lex_string(start),
            '[' | ']' | '{' | '}' | '`' | '\'' | ',' => {
                // No meaning in the Stage 0 skin: brackets are not
                // delimiters here and no reader macros exist —
                // fail-closed instead of silently lexing weird symbols.
                Err(ReadError::UnexpectedCharacter {
                    character: first,
                    at: ByteOffset(start as u32),
                })
            }
            _ => self.lex_atom(start),
        }
    }

    // string
    fn lex_string(&mut self, start: usize) -> Result<Token, ReadError> {
        self.advance('"');
        let mut value = String::new();
        loop {
            let Some(ch) = self.peek_char() else {
                return Err(ReadError::UnterminatedString {
                    started_at: ByteOffset(start as u32),
                });
            };
            match ch {
                '"' => {
                    self.advance('"');
                    return Ok(Token::new(
                        TokenKind::StringLiteral(value),
                        span(self.file_id, start, self.pos),
                    ));
                }
                '\\' => {
                    self.advance('\\');
                    let escape_at = self.pos;
                    let Some(escaped) = self.peek_char() else {
                        return Err(ReadError::UnterminatedString {
                            started_at: ByteOffset(start as u32),
                        });
                    };
                    let decoded = match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => {
                            return Err(ReadError::InvalidEscape {
                                escape: other,
                                at: ByteOffset(escape_at as u32),
                            });
                        }
                    };
                    value.push(decoded);
                    self.advance(escaped);
                }
                _ => {
                    value.push(ch);
                    self.advance(ch);
                }
            }
        }
    }

    fn lex_atom(&mut self, start: usize) -> Result<Token, ReadError> {
        let mut text = String::new();
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || ch == '\u{feff}' || matches!(ch, '(' | ')' | '"' | ';') {
                break;
            }
            text.push(ch);
            self.advance(ch);
        }
        let span = span(self.file_id, start, self.pos);
        match classify_atom(&text)? {
            Some(kind) => Ok(Token::new(kind, span)),
            None => unreachable!("atom run always consumes at least one character"),
        }
    }
}

fn span(file_id: FileId, start: usize, end: usize) -> Span {
    Span::new(
        file_id,
        ByteOffset(start as u32),
        ByteOffset(end as u32),
        ExpansionId::ROOT,
    )
    .expect("lexer spans are well-ordered by construction")
}

fn is_number_shaped(text: &str) -> bool {
    let bytes = text.as_bytes();
    let first = bytes[0];
    if first.is_ascii_digit() {
        return true;
    }
    if matches!(first, b'+' | b'-' | b'.') {
        return bytes.len() > 1 && bytes[1].is_ascii_digit();
    }
    false
}

fn classify_atom(text: &str) -> Result<Option<TokenKind>, ReadError> {
    if text.is_empty() {
        return Ok(None);
    }
    // bool
    if text.starts_with('#') {
        return match text {
            "#t" => Ok(Some(TokenKind::BoolLiteral(true))),
            "#f" => Ok(Some(TokenKind::BoolLiteral(false))),
            _ => Err(ReadError::InvalidBool {
                text: text.to_owned(),
            }),
        };
    }
    // number
    if is_number_shaped(text) {
        if let Ok(v) = text.parse::<i64>() {
            return Ok(Some(TokenKind::IntLiteral(v)));
        }
        if let Ok(v) = text.parse::<f64>() {
            return Ok(Some(TokenKind::FloatLiteral(v)));
        }
        return Err(ReadError::MalformedNumber {
            text: text.to_owned(),
        });
    }
    // ident
    match Symbol::new(text) {
        Ok(symbol) => Ok(Some(TokenKind::Identifier(symbol))),
        Err(_) => Err(ReadError::InvalidSymbol {
            text: text.to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the lexer feature point (sub-stage test doc FP1).
    //! CJK fixture note (SOP §10.5 registered whitelist): the
    //! `symbols_accept_unicode_cjk` case uses CJK *as the tested
    //! object* (atom acceptance), which is the sanctioned fixture
    //! category.

    use super::*;
    use crate::token::TokenKind as K;

    fn file() -> FileId {
        FileId(0)
    }

    fn kinds(src: &str) -> Vec<TokenKind> {
        lex(file(), src)
            .expect("lexing must succeed")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    // ---------- positive ----------

    #[test]
    fn lexes_mixed_program_all_kinds() {
        let tokens = lex(file(), "(add 1 2.5 #t \"s\")").expect("mixed program lexes");
        let kinds: Vec<&str> = tokens.iter().map(|t| t.kind_name()).collect();
        assert_eq!(
            kinds,
            vec![
                "delimiter",
                "identifier",
                "int",
                "float",
                "bool",
                "string",
                "delimiter",
                "eof"
            ]
        );
        let spans: Vec<(u32, u32)> = tokens
            .iter()
            .map(|t| (t.span.start.0, t.span.end.0))
            .collect();
        assert_eq!(
            spans,
            vec![
                (0, 1),
                (1, 4),
                (5, 6),
                (7, 10),
                (11, 13),
                (14, 17),
                (17, 18),
                (18, 18)
            ]
        );
    }

    #[test]
    fn numbers_follow_rust_parse_semantics() {
        let kinds = kinds(".5 1. 1e5 +3 -7 42");
        assert_eq!(
            kinds,
            vec![
                K::FloatLiteral(0.5),
                K::FloatLiteral(1.0),
                K::FloatLiteral(100000.0),
                K::IntLiteral(3),
                K::IntLiteral(-7),
                K::IntLiteral(42),
                K::Eof
            ]
        );
    }

    #[test]
    fn symbols_accept_unicode_cjk_and_specials() {
        // Symbols that must NOT be number-shaped: `+`, `-`, `...`,
        // `nil?`, `set!`, CJK identifiers.
        let kinds = kinds("+ - ... nil? set! 计数器");
        let names: Vec<&str> = kinds
            .iter()
            .map(|k| match k {
                K::Identifier(s) => s.as_str(),
                K::Eof => "eof",
                _ => "other",
            })
            .collect();
        assert_eq!(
            names,
            vec!["+", "-", "...", "nil?", "set!", "计数器", "eof"]
        );
    }

    #[test]
    fn comments_and_whitespace_are_skipped_losslessly() {
        let src = "; header\n ( f ; inline\n 1 ) ; tail";
        let tokens = lex(file(), src).expect("commented source lexes");
        let names: Vec<&str> = tokens.iter().map(|t| t.kind_name()).collect();
        assert_eq!(
            names,
            vec!["delimiter", "identifier", "int", "delimiter", "eof"]
        );
        // Lossless coverage: the slice between consecutive token spans
        // is exactly the skipped trivia — whitespace runs and `;`
        // comments (possibly several, possibly trailing without a
        // newline).
        for pair in tokens.windows(2) {
            let gap = &src[bytes_of(src, pair[0].span.end.0)..bytes_of(src, pair[1].span.start.0)];
            let mut rest = gap;
            loop {
                let trimmed = rest.trim_start();
                if trimmed.is_empty() {
                    break;
                }
                assert!(
                    trimmed.starts_with(';'),
                    "gap must be whitespace or comment trivia, got {gap:?}"
                );
                let line_end = trimmed.find('\n').unwrap_or(trimmed.len());
                rest = &trimmed[line_end..];
            }
        }
    }

    /// Map a byte offset back to a `str` slice index for gap inspection
    /// (offsets in this fixture are ASCII-only, so index == offset).
    fn bytes_of(src: &str, offset: u32) -> usize {
        let idx = offset as usize;
        assert!(src.is_char_boundary(idx), "fixture is ASCII at {idx}");
        idx
    }

    #[test]
    fn string_escapes_decode() {
        // Source: "a\n\t\r\\\"b" written as a Rust escaped literal.
        let kinds = kinds("\"a\\n\\t\\r\\\\\\\"b\"");
        assert_eq!(
            kinds,
            vec![K::StringLiteral("a\n\t\r\\\"b".to_owned()), K::Eof]
        );
    }

    #[test]
    fn eof_token_is_empty_and_bom_is_skipped() {
        let tokens = lex(file(), "\u{feff}x\u{feff}").expect("BOM-only trivia lexes");
        // BOM + `x` + BOM: both BOMs skip, `x` is the only token.
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].kind_name(), "identifier");
        assert_eq!((tokens[0].span.start.0, tokens[0].span.end.0), (3, 4));
        assert_eq!(tokens[1].kind, K::Eof);
        assert!(tokens[1].span.is_empty());
        // Empty source: just Eof.
        let empty = lex(file(), "").expect("empty source lexes");
        assert_eq!(empty.len(), 1);
        assert_eq!(empty[0].kind, K::Eof);
    }

    // ---------- negative ----------

    #[test]
    fn rejects_unterminated_string() {
        let err = lex(file(), "\"abc").expect_err("unterminated string must fail");
        assert_eq!(
            err,
            ReadError::UnterminatedString {
                started_at: ByteOffset(0)
            }
        );
    }

    #[test]
    fn rejects_invalid_escape() {
        let err = lex(file(), "\"a\\x\"").expect_err("unknown escape must fail");
        assert_eq!(
            err,
            ReadError::InvalidEscape {
                escape: 'x',
                at: ByteOffset(3)
            }
        );
    }

    #[test]
    fn rejects_unknown_hash_atom() {
        let err = lex(file(), "#q").expect_err("unknown hash atom must fail");
        assert_eq!(
            err,
            ReadError::InvalidBool {
                text: "#q".to_owned()
            }
        );
    }

    #[test]
    fn rejects_long_bool_spelling() {
        // The Stage 0 skin defines exactly `#t` / `#f` — `#true` is
        // fail-closed, not silently accepted.
        let err = lex(file(), "#true").expect_err("long bool spelling must fail");
        assert_eq!(
            err,
            ReadError::InvalidBool {
                text: "#true".to_owned()
            }
        );
    }

    #[test]
    fn rejects_multi_dot_number() {
        let err = lex(file(), "1.2.3").expect_err("multi-dot number must fail");
        assert_eq!(
            err,
            ReadError::MalformedNumber {
                text: "1.2.3".to_owned()
            }
        );
    }

    #[test]
    fn rejects_digit_prefixed_symbol() {
        let err = lex(file(), "12abc").expect_err("digit-prefixed symbol must fail");
        assert_eq!(
            err,
            ReadError::MalformedNumber {
                text: "12abc".to_owned()
            }
        );
    }

    #[test]
    fn rejects_hex_prefixed_number() {
        let err = lex(file(), "0x1F").expect_err("hex literal must fail");
        assert_eq!(
            err,
            ReadError::MalformedNumber {
                text: "0x1F".to_owned()
            }
        );
    }

    #[test]
    fn rejects_underscored_number() {
        let err = lex(file(), "1_000").expect_err("underscored number must fail");
        assert_eq!(
            err,
            ReadError::MalformedNumber {
                text: "1_000".to_owned()
            }
        );
    }

    #[test]
    fn rejects_dangling_exponent() {
        let err = lex(file(), "1e").expect_err("dangling exponent must fail");
        assert_eq!(
            err,
            ReadError::MalformedNumber {
                text: "1e".to_owned()
            }
        );
    }

    #[test]
    fn rejects_control_char_atom() {
        let err = lex(file(), "a\u{0}b").expect_err("control char atom must fail");
        assert_eq!(
            err,
            ReadError::InvalidSymbol {
                text: "a\u{0}b".to_owned()
            }
        );
    }

    #[test]
    fn rejects_bracket_characters() {
        for src in ["[", "]", "{", "}"] {
            let ch = src.chars().next().expect("one char");
            let err = lex(file(), src).expect_err("brackets have no Stage 0 meaning");
            assert_eq!(
                err,
                ReadError::UnexpectedCharacter {
                    character: ch,
                    at: ByteOffset(0)
                }
            );
        }
    }

    #[test]
    fn rejects_reader_macro_characters() {
        for src in ["`", "'", ","] {
            let ch = src.chars().next().expect("one char");
            let err = lex(file(), src).expect_err("reader macros do not exist in Stage 0");
            assert_eq!(
                err,
                ReadError::UnexpectedCharacter {
                    character: ch,
                    at: ByteOffset(0)
                }
            );
        }
    }

    #[test]
    fn rejects_lone_hash_atom() {
        // `#` alone starts a hash atom but matches neither `#t` nor
        // `#f` — fail-closed, not a symbol.
        let err = lex(file(), "#").expect_err("lone hash must fail");
        assert_eq!(
            err,
            ReadError::InvalidBool {
                text: "#".to_owned()
            }
        );
    }

    #[test]
    fn rejects_escape_at_end_of_input() {
        // `"a\` — the escape backslash runs into end of input: the
        // string (already open) is unterminated.
        let err = lex(file(), "\"a\\").expect_err("escape at end of input must fail");
        assert_eq!(
            err,
            ReadError::UnterminatedString {
                started_at: ByteOffset(0)
            }
        );
    }

    #[test]
    fn atom_boundaries_are_lisp_terminators() {
        // `a"b"` = symbol `a` followed by string "b"; `x;y` = symbol `x`
        // then a comment — positive adjacency check that also pins the
        // terminator set used by the negative cases above.
        let kinds = kinds("a\"b\"");
        assert_eq!(
            kinds,
            vec![
                K::Identifier(Symbol::new("a").expect("valid symbol")),
                K::StringLiteral("b".to_owned()),
                K::Eof
            ]
        );
        let tokens = lex(file(), "x;y\nz").expect("comment-adjacent atoms lex");
        let names: Vec<&str> = tokens.iter().map(|t| t.kind_name()).collect();
        assert_eq!(names, vec!["identifier", "identifier", "eof"]);
    }
}
