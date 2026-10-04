//! Stage 0.0 plan tests — karst-span seed MUV, cross-crate integration level.
// Shared helpers: mounted once by the runner entry (tests/runner.rs) and
// imported here via `super::`.
use super::common::{close_token, covering_span, ident_token, open_token, root_span};
use karst::span::{ByteOffset, ExpansionId, FileId, Span, SpanError};

#[test]
fn workspace_aggregate_reexport_is_consumable() {
    // Positive: the root aggregation crate (`karst`) re-exports the member
    // crate surface (SOP §8.4.6 cross-crate layer + §10.1 rule 4).
    let span =
        Span::root(FileId(0), ByteOffset(2), ByteOffset(8)).expect("valid span via aggregate path");
    assert_eq!(span.to_string(), "file#0:2..8");
    assert_eq!(span.expansion_id, ExpansionId::ROOT);
    // Version tracks the workspace manifest (0.1 design-adjudications
    // erratum: self-consistent with the root package, no hard-coded
    // literal to churn on every sub-stage bump).
    assert_eq!(karst::WORKSPACE_VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn pipeline_seed_token_span_reduction_pattern() {
    // Positive + cross-stage consumption: a reader-shaped consumer folds
    // token spans into a node span via `join` (docs/stage0.md §12.1
    // consumption table — the contract Stage 0.3 will rely on).
    let tokens = [
        open_token(root_span(0, 0, 1)),
        ident_token("lambda", root_span(0, 1, 7)),
        ident_token("x", root_span(0, 8, 11)),
        ident_token("x", root_span(0, 12, 13)),
        close_token(root_span(0, 13, 14)),
    ];
    let covering = covering_span(&tokens).expect("same-file token spans must reduce");
    assert_eq!((covering.start.0, covering.end.0), (0, 14));
    assert_eq!(tokens[0].kind.to_string(), "(");
}

#[test]
fn aggregate_path_rejects_reversed_span() {
    // Negative: constructor invariant holds through the aggregate path.
    let err = Span::root(FileId(3), ByteOffset(12), ByteOffset(4))
        .expect_err("reversed span must be rejected via aggregate path");
    assert!(matches!(err, SpanError::StartAfterEnd { .. }));
}

#[test]
fn aggregate_path_join_rejects_cross_file() {
    // Negative: file-identity invariant holds through the aggregate path.
    let a = root_span(0, 0, 10);
    let b = root_span(1, 0, 10);
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
fn aggregate_path_join_rejects_cross_generation() {
    // Negative: expansion-generation invariant holds through the aggregate
    // path (TD-002 deferral boundary is visible to consumers).
    let a = Span::new(FileId(0), ByteOffset(0), ByteOffset(10), ExpansionId(0)).unwrap();
    let b = Span::new(FileId(0), ByteOffset(20), ByteOffset(30), ExpansionId(3)).unwrap();
    assert!(matches!(
        a.join(&b),
        Err(SpanError::JoinExpansionMismatch { .. })
    ));
}

#[test]
fn aggregate_path_contains_respects_exclusive_end() {
    // Negative: half-open semantics hold through the aggregate path.
    let span = root_span(0, 10, 20);
    assert!(!span.contains_offset(ByteOffset(20)));
    assert!(!span.contains_offset(ByteOffset(9)));
}

#[test]
fn aggregate_path_error_display_is_stable() {
    // Negative: error rendering contract holds through the aggregate path.
    let err = SpanError::StartAfterEnd {
        start: ByteOffset(9),
        end: ByteOffset(2),
    };
    assert_eq!(
        err.to_string(),
        "span start offset 9 is greater than end offset 2"
    );
}

#[test]
fn token_pattern_propagates_member_construction_errors() {
    // Negative + cross-stage consumption: a consumer folding malformed
    // member spans must see the constructor error, not a silent span.
    let bad = Span::root(FileId(0), ByteOffset(30), ByteOffset(5));
    let tokens = [
        open_token(root_span(0, 0, 1)),
        ident_token("lambda", root_span(0, 1, 7)),
    ];
    assert!(
        bad.is_err(),
        "malformed member span must be rejected before folding"
    );
    let covering = covering_span(&tokens).expect("well-formed tokens must reduce");
    assert_eq!(covering.end, ByteOffset(7));
}
