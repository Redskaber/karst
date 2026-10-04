//! @path: karst/crates/karst_reader/error.rs
//! @author: redskaber
//! @datetime: 2026-10-02
//! @discription: karst::crates::karst_reader::error

use std::error::Error;
use std::fmt;

use karst_span::ByteOffset;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    // lexer
    UnexpectedCharacter { character: char, at: ByteOffset },
    UnterminatedString { started_at: ByteOffset },
    InvalidEscape { escape: char, at: ByteOffset },
    InvalidBool { text: String },
    MalformedNumber { text: String },
    InvalidSymbol { text: String },
    // parser
    NestingDepthExceeded { depth: usize, limit: usize },
    UnclosedList { opened_at: ByteOffset },
    EmptyApplication { at: ByteOffset },
    UnexpectedClose { at: ByteOffset },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::UnexpectedCharacter { character, at } => {
                write!(f, "unexpected character `{character}` at byte {}", at.0)
            }
            ReadError::UnterminatedString { started_at } => {
                write!(
                    f,
                    "unterminated string literal starting at byte {}",
                    started_at.0
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
            ReadError::UnclosedList { opened_at } => {
                write!(
                    f,
                    "unclosed delimiter: `(` at byte {} is never closed",
                    opened_at.0
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
    //! Unit tests for the ReadError rendering surface (sub-stage test
    //! doc FP3; R8 message-shape assertions).

    use super::*;

    fn every_variant() -> Vec<ReadError> {
        vec![
            ReadError::UnclosedList {
                opened_at: ByteOffset(3),
            },
            ReadError::UnexpectedClose { at: ByteOffset(7) },
            ReadError::UnterminatedString {
                started_at: ByteOffset(0),
            },
            ReadError::InvalidEscape {
                escape: 'x',
                at: ByteOffset(4),
            },
            ReadError::InvalidBool {
                text: "#q".to_owned(),
            },
            ReadError::MalformedNumber {
                text: "1.2.3".to_owned(),
            },
            ReadError::InvalidSymbol {
                text: "a\u{0}b".to_owned(),
            },
            ReadError::EmptyApplication { at: ByteOffset(0) },
            ReadError::NestingDepthExceeded {
                depth: 257,
                limit: 256,
            },
            ReadError::UnexpectedCharacter {
                character: '[',
                at: ByteOffset(2),
            },
        ]
    }

    // ---------- negative (error surface) ----------

    #[test]
    fn display_family_follows_rust_message_conventions() {
        // R8 family check over every variant: lowercase start, no
        // trailing period.
        for err in every_variant() {
            let rendered = err.to_string();
            let first = rendered.chars().next().expect("non-empty message");
            assert!(
                first.is_lowercase(),
                "message must start lowercase: {rendered}"
            );
            assert!(!rendered.ends_with('.'), "no trailing period: {rendered}");
        }
    }

    #[test]
    fn display_unclosed_mentions_byte_position() {
        let err = ReadError::UnclosedList {
            opened_at: ByteOffset(12),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("12"));
        assert!(rendered.contains("never closed"));
        assert!(rendered.contains("`(`"));
    }

    #[test]
    fn display_invalid_symbol_is_byte_identical_across_layers() {
        // Single-source dictionary (SOP §10.5 / R8): the same source
        // concept renders the same English text in every layer.
        let reader = ReadError::InvalidSymbol {
            text: "a b".to_owned(),
        }
        .to_string();
        let syntax = karst_syntax::SyntaxError::InvalidSymbol {
            text: "a b".to_owned(),
        }
        .to_string();
        assert_eq!(reader, syntax);
    }

    #[test]
    fn display_invalid_bool_mentions_expected_literals() {
        let err = ReadError::InvalidBool {
            text: "#true".to_owned(),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("`#true`"));
        assert!(rendered.contains("`#t`"));
        assert!(rendered.contains("`#f`"));
    }

    #[test]
    fn error_variants_distinguishable_by_equality() {
        let variants = every_variant();
        for (i, a) in variants.iter().enumerate() {
            for (j, b) in variants.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "variants {i} and {j} must be distinct");
                }
            }
        }
        // Same variant with same payload compares equal.
        assert_eq!(variants[0], variants[0].clone());
    }
}
