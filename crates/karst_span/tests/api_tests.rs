//! karst-span crate-level integration tests: verify the public API surface

use karst_span::{ByteOffset, ExpansionId, FileId, Span, SpanError};

#[test]
fn public_api_exports_documented_surface() {
    // Compile-level surface check: every documented name is reachable from
    // the crate root (SOP §10.1 rule 4 explicit re-export list).
    let span = Span::root(FileId(0), ByteOffset(1), ByteOffset(2)).unwrap();
    assert_eq!(span.expansion_id, ExpansionId::ROOT);
    let _err: SpanError = SpanError::StartAfterEnd {
        start: ByteOffset(2),
        end: ByteOffset(1),
    };
}

#[test]
fn public_constructor_and_join_roundtrip() {
    let a = Span::new(FileId(0), ByteOffset(0), ByteOffset(5), ExpansionId::ROOT).unwrap();
    let b = Span::new(FileId(0), ByteOffset(5), ByteOffset(10), ExpansionId::ROOT).unwrap();
    let joined = a
        .join(&b)
        .expect("adjacent spans in one file/generation must join");
    assert_eq!((joined.start.0, joined.end.0), (0, 10));
}

#[test]
fn public_constructor_rejects_reversed_bounds() {
    let err = Span::new(FileId(0), ByteOffset(9), ByteOffset(3), ExpansionId::ROOT)
        .expect_err("reversed bounds must be rejected via the public path");
    assert!(matches!(err, SpanError::StartAfterEnd { .. }));
}

#[test]
fn public_join_rejects_cross_file() {
    let a = Span::root(FileId(0), ByteOffset(0), ByteOffset(4)).unwrap();
    let b = Span::root(FileId(5), ByteOffset(0), ByteOffset(4)).unwrap();
    assert!(matches!(
        a.join(&b),
        Err(SpanError::JoinFileMismatch { .. })
    ));
}

#[test]
fn public_join_rejects_cross_generation() {
    let a = Span::new(FileId(0), ByteOffset(0), ByteOffset(4), ExpansionId(0)).unwrap();
    let b = Span::new(FileId(0), ByteOffset(8), ByteOffset(12), ExpansionId(2)).unwrap();
    assert!(matches!(
        a.join(&b),
        Err(SpanError::JoinExpansionMismatch { .. })
    ));
}

#[test]
fn public_root_rejects_invalid_bounds() {
    assert!(Span::root(FileId(0), ByteOffset(10), ByteOffset(2)).is_err());
}

#[test]
fn public_error_display_via_reexport() {
    let err = SpanError::JoinFileMismatch {
        lhs: FileId(0),
        rhs: FileId(7),
    };
    assert_eq!(
        err.to_string(),
        "cannot join spans from different files (file#0 vs file#7)"
    );
}

#[test]
fn public_error_is_std_error() {
    fn assert_error<E: std::error::Error>(_: &E) {}
    let err = SpanError::JoinExpansionMismatch {
        lhs: ExpansionId(1),
        rhs: ExpansionId(2),
    };
    assert_error(&err);
}
