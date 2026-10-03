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
            '[' | ']' | '{' | '}' | '`' | '\'' | ',' => Err(ReadError::UnexpctedCharacter {
                ch: first,
                at: ByteOffset(start as u32),
            }),
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
                    start_at: ByteOffset(start as u32),
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
                            start_at: ByteOffset(start as u32),
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
            return Ok(Some(TokenKind::IntLitral(v)));
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
    // more ...
}
