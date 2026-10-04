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
    pub fn contains_offset(&self, offset: ByteOffset) -> bool {
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
                    "cannot join spans from different expansion generations ({} vs {})",
                    lhs.0, rhs.0
                )
            }
        }
    }
}

impl Error for SpanError {}

#[cfg(test)]
mod tests {
    //! Unit tests, organized by feature point (FP1-FP6); every feature point
    use super::*;

    // ---------- FP1: Span::new checked constructor ----------

    #[test]
    fn new_valid_span_roundtrips_fields() {
        let span = Span::new(FileId(7), ByteOffset(10), ByteOffset(20), ExpansionId(3))
            .expect("valid span must construct");
        assert_eq!(span.file_id, FileId(7));
        assert_eq!(span.start, ByteOffset(10));
        assert_eq!(span.end, ByteOffset(20));
        assert_eq!(span.expansion_id, ExpansionId(3));
        // Empty boundary: start == end is legal.
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

    #[test]
    fn new_rejects_start_after_end() {
        let err = Span::new(FileId(0), ByteOffset(10), ByteOffset(9), ExpansionId::ROOT)
            .expect_err("start > end must be rejected");
        assert_eq!(
            err,
            SpanError::StartAfterEnd {
                start: ByteOffset(10),
                end: ByteOffset(9)
            }
        );
    }

    #[test]
    fn new_rejects_reversed_adjacent_offsets() {
        let err = Span::new(FileId(0), ByteOffset(1), ByteOffset(0), ExpansionId::ROOT)
            .expect_err("reversed adjacent offsets must be rejected");
        assert!(matches!(err, SpanError::StartAfterEnd { .. }));
    }

    #[test]
    fn new_rejects_reversal_far_apart() {
        let err = Span::new(FileId(2), ByteOffset(1000), ByteOffset(0), ExpansionId(1))
            .expect_err("far-apart reversal must be rejected");
        assert_eq!(
            err,
            SpanError::StartAfterEnd {
                start: ByteOffset(1000),
                end: ByteOffset(0)
            }
        );
    }

    #[test]
    fn new_rejects_reversal_at_u32_max() {
        let err = Span::new(
            FileId(0),
            ByteOffset(u32::MAX),
            ByteOffset(u32::MAX - 1),
            ExpansionId::ROOT,
        )
        .expect_err("reversal at u32::MAX boundary must be rejected");
        assert!(matches!(err, SpanError::StartAfterEnd { .. }));
    }

    // ---------- FP2: Span::root shorthand ----------

    #[test]
    fn root_shorthand_uses_root_generation_and_validates() {
        let via_root = Span::root(FileId(4), ByteOffset(3), ByteOffset(8))
            .expect("valid root span must construct");
        let via_new = Span::new(FileId(4), ByteOffset(3), ByteOffset(8), ExpansionId::ROOT)
            .expect("valid explicit span must construct");
        assert_eq!(via_root, via_new);
        assert_eq!(via_root.expansion_id, ExpansionId::ROOT);
    }

    #[test]
    fn root_rejects_start_after_end() {
        let err = Span::root(FileId(0), ByteOffset(5), ByteOffset(4))
            .expect_err("root shorthand must not bypass validation");
        assert!(matches!(err, SpanError::StartAfterEnd { .. }));
    }

    #[test]
    fn root_error_variant_matches_new_shape() {
        let root_err =
            Span::root(FileId(1), ByteOffset(30), ByteOffset(10)).expect_err("root error expected");
        let new_err = Span::new(FileId(1), ByteOffset(30), ByteOffset(10), ExpansionId::ROOT)
            .expect_err("new error expected");
        assert_eq!(root_err, new_err);
    }

    // ---------- FP3: extent predicates (is_empty / len) ----------

    #[test]
    fn len_and_is_empty_report_extent() {
        let empty = Span::root(FileId(0), ByteOffset(5), ByteOffset(5)).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
        let filled = Span::root(FileId(0), ByteOffset(5), ByteOffset(25)).unwrap();
        assert_eq!(filled.len(), 20);
    }

    #[test]
    fn is_empty_false_for_single_byte_span() {
        let span = Span::root(FileId(0), ByteOffset(4), ByteOffset(5)).unwrap();
        assert!(!span.is_empty());
    }

    #[test]
    fn is_empty_false_for_multibyte_span() {
        let span = Span::root(FileId(0), ByteOffset(0), ByteOffset(u32::MAX)).unwrap();
        assert!(!span.is_empty());
        assert_eq!(span.len(), u32::MAX);
    }

    #[test]
    fn len_nonzero_for_nonempty_span() {
        let span = Span::root(FileId(3), ByteOffset(100), ByteOffset(101)).unwrap();
        assert!(!span.is_empty());
        assert_eq!(span.len(), 1);
    }

    // ---------- FP4: contains_offset (half-open semantics) ----------

    #[test]
    fn contains_offset_true_inside_and_at_start() {
        let span = Span::root(FileId(0), ByteOffset(10), ByteOffset(20)).unwrap();
        assert!(span.contains_offset(ByteOffset(10))); // start is inclusive
        assert!(span.contains_offset(ByteOffset(15))); // interior
        assert!(span.contains_offset(ByteOffset(19))); // end-1, still inside
    }

    #[test]
    fn contains_offset_false_before_start() {
        let span = Span::root(FileId(0), ByteOffset(10), ByteOffset(20)).unwrap();
        assert!(!span.contains_offset(ByteOffset(0)));
    }

    #[test]
    fn contains_offset_false_at_start_minus_one() {
        let span = Span::root(FileId(0), ByteOffset(10), ByteOffset(20)).unwrap();
        assert!(!span.contains_offset(ByteOffset(9)));
    }

    #[test]
    fn contains_offset_false_at_end_exclusive() {
        let span = Span::root(FileId(0), ByteOffset(10), ByteOffset(20)).unwrap();
        assert!(!span.contains_offset(ByteOffset(20)));
    }

    #[test]
    fn contains_offset_false_beyond_end() {
        let span = Span::root(FileId(0), ByteOffset(10), ByteOffset(20)).unwrap();
        assert!(!span.contains_offset(ByteOffset(u32::MAX)));
    }

    #[test]
    fn contains_offset_false_for_empty_span_at_own_offset() {
        let span = Span::root(FileId(0), ByteOffset(7), ByteOffset(7)).unwrap();
        assert!(!span.contains_offset(ByteOffset(7)));
    }

    // ---------- FP5: Span::join ----------

    #[test]
    fn join_covers_disjoint_ranges_symmetrically() {
        let left = Span::new(FileId(0), ByteOffset(10), ByteOffset(20), ExpansionId(2)).unwrap();
        let right = Span::new(FileId(0), ByteOffset(30), ByteOffset(40), ExpansionId(2)).unwrap();
        let joined = left.join(&right).expect("same file+generation must join");
        assert_eq!(joined.start, ByteOffset(10));
        assert_eq!(joined.end, ByteOffset(40));
        assert_eq!(joined.file_id, FileId(0));
        assert_eq!(joined.expansion_id, ExpansionId(2));
        // Symmetry: join order must not matter.
        assert_eq!(left.join(&right), right.join(&left));
        // Nested operand: covering span still spans the outer range.
        let inner = Span::new(FileId(0), ByteOffset(12), ByteOffset(18), ExpansionId(2)).unwrap();
        let nested = left.join(&inner).unwrap();
        assert_eq!((nested.start, nested.end), (ByteOffset(10), ByteOffset(20)));
        // Empty operand at the edge: covering range is unchanged.
        let edge_empty =
            Span::new(FileId(0), ByteOffset(20), ByteOffset(20), ExpansionId(2)).unwrap();
        let with_empty = left.join(&edge_empty).unwrap();
        assert_eq!(
            (with_empty.start, with_empty.end),
            (ByteOffset(10), ByteOffset(20))
        );
    }

    #[test]
    fn join_rejects_different_files() {
        let a = Span::root(FileId(0), ByteOffset(0), ByteOffset(10)).unwrap();
        let b = Span::root(FileId(1), ByteOffset(0), ByteOffset(10)).unwrap();
        let err = a.join(&b).expect_err("cross-file join must be rejected");
        assert_eq!(
            err,
            SpanError::JoinFileMismatch {
                lhs: FileId(0),
                rhs: FileId(1)
            }
        );
    }

    #[test]
    fn join_rejects_overlapping_ranges_across_files() {
        let a = Span::root(FileId(3), ByteOffset(0), ByteOffset(50)).unwrap();
        let b = Span::root(FileId(9), ByteOffset(25), ByteOffset(75)).unwrap();
        let err = a
            .join(&b)
            .expect_err("overlapping ranges must not bypass the file check");
        assert!(matches!(err, SpanError::JoinFileMismatch { .. }));
    }

    #[test]
    fn join_rejects_different_expansion_generations() {
        let a = Span::new(FileId(0), ByteOffset(0), ByteOffset(10), ExpansionId(0)).unwrap();
        let b = Span::new(FileId(0), ByteOffset(20), ByteOffset(30), ExpansionId(1)).unwrap();
        let err = a
            .join(&b)
            .expect_err("cross-generation join must be rejected");
        assert_eq!(
            err,
            SpanError::JoinExpansionMismatch {
                lhs: ExpansionId(0),
                rhs: ExpansionId(1)
            }
        );
    }

    #[test]
    fn join_rejects_identical_ranges_across_generations() {
        let a = Span::new(FileId(0), ByteOffset(5), ByteOffset(15), ExpansionId(0)).unwrap();
        let b = Span::new(FileId(0), ByteOffset(5), ByteOffset(15), ExpansionId(7)).unwrap();
        let err = a
            .join(&b)
            .expect_err("identical ranges must not bypass the generation check");
        assert!(matches!(err, SpanError::JoinExpansionMismatch { .. }));
    }

    #[test]
    fn join_rejects_file_and_expansion_mismatch_reporting_file_first() {
        let a = Span::new(FileId(0), ByteOffset(0), ByteOffset(10), ExpansionId(0)).unwrap();
        let b = Span::new(FileId(1), ByteOffset(0), ByteOffset(10), ExpansionId(4)).unwrap();
        let err = a.join(&b).expect_err("double mismatch must be rejected");
        // File identity is the first axis checked (documented precedence).
        assert!(matches!(err, SpanError::JoinFileMismatch { .. }));
    }

    #[test]
    fn join_failure_leaves_operands_unchanged() {
        let a = Span::root(FileId(0), ByteOffset(0), ByteOffset(10)).unwrap();
        let b = Span::root(FileId(1), ByteOffset(20), ByteOffset(30)).unwrap();
        let _ = a.join(&b);
        assert_eq!(
            a,
            Span::root(FileId(0), ByteOffset(0), ByteOffset(10)).unwrap()
        );
        assert_eq!(
            b,
            Span::root(FileId(1), ByteOffset(20), ByteOffset(30)).unwrap()
        );
    }

    // ---------- FP6: Display + error surface ----------

    #[test]
    fn display_renders_file_and_byte_range() {
        let span = Span::new(FileId(2), ByteOffset(8), ByteOffset(16), ExpansionId(5)).unwrap();
        assert_eq!(span.to_string(), "file#2:8..16");
    }

    #[test]
    fn span_error_implements_std_error_trait() {
        let err = Span::root(FileId(0), ByteOffset(1), ByteOffset(0)).unwrap_err();
        let boxed: Box<dyn Error> = Box::new(err);
        assert_eq!(
            boxed.to_string(),
            "span start offset 1 is greater than end offset 0"
        );
    }

    #[test]
    fn error_display_start_after_end_message() {
        let err = SpanError::StartAfterEnd {
            start: ByteOffset(42),
            end: ByteOffset(17),
        };
        assert_eq!(
            err.to_string(),
            "span start offset 42 is greater than end offset 17"
        );
    }

    #[test]
    fn error_display_join_file_mismatch_message() {
        let err = SpanError::JoinFileMismatch {
            lhs: FileId(1),
            rhs: FileId(2),
        };
        assert_eq!(
            err.to_string(),
            "cannot join spans from different files (file#1 vs file#2)"
        );
    }

    #[test]
    fn error_display_join_expansion_mismatch_message() {
        let err = SpanError::JoinExpansionMismatch {
            lhs: ExpansionId(0),
            rhs: ExpansionId(3),
        };
        assert_eq!(
            err.to_string(),
            "cannot join spans from different expansion generations (0 vs 3)"
        );
    }

    #[test]
    fn error_display_follows_rust_message_conventions() {
        // SOP §10.5 (R8): lowercase start, no trailing period, ASCII punctuation.
        let samples = [
            SpanError::StartAfterEnd {
                start: ByteOffset(2),
                end: ByteOffset(1),
            }
            .to_string(),
            SpanError::JoinFileMismatch {
                lhs: FileId(0),
                rhs: FileId(1),
            }
            .to_string(),
            SpanError::JoinExpansionMismatch {
                lhs: ExpansionId(0),
                rhs: ExpansionId(1),
            }
            .to_string(),
        ];
        for message in samples {
            assert!(
                !message.ends_with('.'),
                "message must not end with a period: {message}"
            );
            assert_eq!(
                message.chars().next().map(char::is_lowercase),
                Some(true),
                "message must start lowercase: {message}"
            );
            assert!(message.is_ascii(), "message must stay ASCII: {message}");
        }
    }
}
