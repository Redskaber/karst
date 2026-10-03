//! @path: karst/crates/karst_reader/error.rs
//! @author: redskaber
//! @datetime: 2026-10-02
//! @discription: karst::crates::karst_reader::error

use core::fmt;
use std::error::Error;

use karst_span::ByteOffset;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    // lexer
    UnexpctedCharacter { ch: char, at: ByteOffset },
    UnterminatedString { start_at: ByteOffset },
    InvalidEscape { escape: char, at: ByteOffset },
    InvalidBool { text: String },
    MalformedNumber { text: String },
    InvalidSymbol { text: String },
    // parser
    NestingDepthExceeded { depth: usize, limit: usize },
    UnclosedList { open_at: ByteOffset },
    EmptyApplication { at: ByteOffset },
    UnexpectedClose { at: ByteOffset },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::UnexpctedCharacter { ch, at } => {
                write!(f, "unexpected character `{ch}` at byte {}", at.0)
            }
            ReadError::UnterminatedString { start_at } => {
                write!(
                    f,
                    "unterminated string literal starting at byte {}",
                    start_at.0
                )
            }
            ReadError::InvalidEscape { escape, at } => {
                write!(f, "invalid string escape `{escape}` at byte {}", at.0)
            }
            ReadError::InvalidBool { text } => {
                write!(f, "invalid boolean literal `{text}`: expected `#t` or `#f`")
            }
            ReadError::MalformedNumber { text } => {
                write!(f, "malformed number literal `{text}`")
            }
            ReadError::InvalidSymbol { text } => {
                write!(
                    f,
                    "invalid symbol `{text}`: must be non-empty with no whitespace or control characters"
                )
            }
            ReadError::NestingDepthExceeded { depth, limit } => {
                write!(f, "nesting depth {depth} exceeds the limit of {limit}")
            }
            ReadError::UnclosedList { open_at } => {
                write!(
                    f,
                    "unclosed delimiter: `(` at byte {} is never closed",
                    open_at.0
                )
            }
            ReadError::EmptyApplication { at } => {
                write!(
                    f,
                    "empty application: `()` at byte {} has no operator",
                    at.0
                )
            }
            ReadError::UnexpectedClose { at } => {
                write!(f, "unexpected closing delimiter `)` at byte {}", at.0)
            }
        }
    }
}

impl Error for ReadError {}

#[cfg(test)]
mod tests {
    // more ...
}
