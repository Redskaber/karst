//! @path: karst/crates/karst_span/span.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span::span
//! Span: source code position unvariable exp
//!   - error position to source code
//!   - debug to use check code -> IR -> source code
//!

use core::fmt;

/// source file id
pub type FileId = u32;

/// bytes offset
pub type ByteOffset = u32;

/// macro expansion id
pub type ExpansionId = u32;

/// source position exp
/// some span: file_id + expansion_id => pin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub file_id: FileId,
    pub start: ByteOffset,
    pub end: ByteOffset,
    pub expansion_id: ExpansionId,
}

impl Span {
    pub fn new(file_id: FileId, start: ByteOffset, end: ByteOffset) -> Self {
        Span {
            file_id,
            start,
            end,
            expansion_id: 0,
        }
    }

    pub fn dummy() -> Self {
        Span::new(0, 0, 0)
    }

    /// up expansion_id
    pub fn bumped_expansion(self) -> Self {
        Self {
            expansion_id: self.expansion_id.saturating_add(1),
            ..self
        }
    }

    /// merge if file_id eq else no thing
    pub fn merge(self, other: Span) -> Span {
        if self.file_id != other.file_id || self.expansion_id != other.expansion_id {
            return self;
        }
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            ..self
        }
    }

    /// byte offset in contains
    /// [start, end)
    pub fn contains(self, offset: ByteOffset) -> bool {
        offset >= self.start && offset < self.end
    }

    pub fn len(self) -> ByteOffset {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(self) -> bool {
        self.end <= self.start
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Span(file {}, {}..{}{}",
            self.file_id,
            self.start,
            self.end,
            if self.expansion_id > 0 {
                format!(", exp {})", self.expansion_id)
            } else {
                ")".to_string()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_merge_union() {
        let a = Span::new(0, 5, 10);
        let b = Span::new(0, 8, 20);
        assert_eq!(a.merge(b), Span::new(0, 5, 20));
        assert_eq!(b.merge(a), Span::new(0, 5, 20));
    }

    #[test]
    fn span_merge_different_files_falls_back() {
        let a = Span::new(0, 5, 10);
        let b = Span::new(1, 0, 3);
        assert_eq!(a.merge(b), a);
    }

    #[test]
    fn span_expansion_bump() {
        let a = Span::new(0, 1, 2);
        assert_eq!(a.bumped_expansion().expansion_id, 1);
        assert_eq!(a.bumped_expansion().bumped_expansion().expansion_id, 2);
    }

    #[test]
    fn span_contains_and_len() {
        let a = Span::new(0, 10, 20);
        assert!(a.contains(10) && a.contains(19));
        assert!(!a.contains(20) && !a.contains(9));
        assert_eq!(a.len(), 10);
        assert!(!a.is_empty());
        assert!(Span::dummy().is_empty());
    }
}
