//! @path: karst/crates/karst_span/span.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span::span
//!
//! `Span`: an immutable representation of a source location.
//!
//! In the style of rustc, every Token / AST node / IR node / bytecode instruction
//! carries a `Span`:
//! - Any error can be precisely located to source;
//! - The debugger can reverse-lookup bytecode -> IR -> source;
//! - Incremental compilation can perform fine-grained invalidation based on `Span`.
//!
//! Column semantics: **byte offset is the primary key; line/column are only for diagnostic rendering**
//! — `Span` stores only byte offsets, and line/column are derived at render time by `SourceMap`.

use core::fmt;
use std::error::Error;

/// Identify of a source file within one compilation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

/// Byte offset into a source file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ByteOffset(pub u32);

/// Macro-expansion generation a piece of syntax originates form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExpansionId(pub u32);

impl ExpansionId {
    /// Generation of syntax written in the source file itself (no macro)
    pub const ROOT: Self = Self(0);
}

/// source position exp
/// some span: file_id + expansion_id => pin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file_id: FileId,
    pub start: ByteOffset,
    pub end: ByteOffset,
    pub expansion_id: ExpansionId,
}

impl Span {
    pub fn new(
        file_id: FileId,
        start: ByteOffset,
        end: ByteOffset,
        expansion_id: ExpansionId,
    ) -> Result<Span, SpanError> {
        // grand start > end
        if start > end {
            return Err(SpanError::StartAfterEnd { start, end });
        }
        Ok(Span {
            file_id,
            start,
            end,
            expansion_id,
        })
    }

    /// Construcator for root-generation syntax
    pub fn root(file_id: FileId, start: ByteOffset, end: ByteOffset) -> Result<Span, SpanError> {
        Span::new(file_id, start, end, ExpansionId::ROOT)
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn len(&self) -> u32 {
        self.end.0 - self.start.0
    }

    /// byte offset in contains
    /// [start, end)
    pub fn contains(&self, offset: ByteOffset) -> bool {
        offset >= self.start && offset < self.end
    }

    /// Smallest span covering both `self` and `other`
    pub fn join(&self, other: &Span) -> Result<Span, SpanError> {
        if self.file_id != other.file_id {
            return Err(SpanError::JoinFileMismatch {
                lhs: self.file_id,
                rhs: other.file_id,
            });
        }
        if self.expansion_id != other.expansion_id {
            return Err(SpanError::JoinExpansionMismatch {
                lhs: self.expansion_id,
                rhs: other.expansion_id,
            });
        }
        Ok(Span {
            file_id: self.file_id,
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            expansion_id: self.expansion_id,
        })
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "file#{}:{}..{}",
            self.file_id.0, self.start.0, self.end.0
        )
    }
}

/// Span error set
/// Errors produced by Span construcation and merging
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanError {
    /// `start` byte offset is greater than `end`
    StartAfterEnd { start: ByteOffset, end: ByteOffset },
    /// [`Span::join`] across two different files
    JoinFileMismatch { lhs: FileId, rhs: FileId },
    /// [`Span::join`] across two macro-expansion generations
    JoinExpansionMismatch { lhs: ExpansionId, rhs: ExpansionId },
}

impl fmt::Display for SpanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanError::StartAfterEnd { start, end } => {
                write!(
                    f,
                    "span start offset {} is greater than end offset {}",
                    start.0, end.0
                )
            }
            SpanError::JoinFileMismatch { lhs, rhs } => {
                write!(
                    f,
                    "cannot join spans from different files (file#{} vs file#{})",
                    lhs.0, rhs.0
                )
            }
            SpanError::JoinExpansionMismatch { lhs, rhs } => {
                write!(
                    f,
                    "cannot join spans different expansion generations ({} vs {})",
                    lhs.0, rhs.0
                )
            }
        }
    }
}

impl Error for SpanError {}

#[cfg(test)]
mod tests {
    use super::*;

    // FP1: Span::new checked Construcator
    #[test]
    fn new_valid_span_roundtrips_fields() {
        let span = Span::new(FileId(7), ByteOffset(10), ByteOffset(20), ExpansionId(3))
            .expect("valid span must construct");
        assert_eq!(span.file_id, FileId(7));
        assert_eq!(span.start, ByteOffset(10));
        assert_eq!(span.end, ByteOffset(20));
        assert_eq!(span.expansion_id, ExpansionId(3));

        let empty = Span::new(FileId(0), ByteOffset(0), ByteOffset(0), ExpansionId::ROOT)
            .expect("empty span at origin must construct");
        assert!(empty.is_empty());
    }

    #[test]
    fn new_accepts_empty_span_at_u32_max() {
        let span = Span::new(
            FileId(1),
            ByteOffset(u32::MAX),
            ByteOffset(u32::MAX),
            ExpansionId::ROOT,
        )
        .expect("empty span at u32::MAX must construct");
        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
    }

    // more ...
}
