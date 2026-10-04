//! Stage 0.2 plan tests — karst-syntax SyntaxObject seed MUV,
//! cross-crate integration level.

// Shared helpers: mounted once by the runner entry (tests/runner.rs) and
// imported here via `super::`.
use super::common::{close_token, covering_span, ident_token, int_token, open_token, root_span};
use karst::WORKSPACE_VERSION;
use karst::span::{ByteOffset, ExpansionId, FileId, Span, SpanError};
use karst::syntax::{Phase, ScopeId, ScopeSet, SyntaxError, SyntaxObject};

#[test]
fn reader_shaped_token_to_stx_assembly() {
    // Positive + cross-stage consumption: a reader-shaped consumer maps
    // real reader tokens to syntax objects — the token
    // spans reduce to the list span (covering_span) and the token text
    // flows through the checked symbol constructor. This is the
    // contract Stage 0.3 will formalize.
    let tokens = [
        open_token(root_span(0, 0, 1)),
        ident_token("add", root_span(0, 1, 4)),
        int_token(1, root_span(0, 5, 6)),
        int_token(2, root_span(0, 7, 8)),
        close_token(root_span(0, 8, 9)),
    ];
    let list_span = covering_span(&tokens).expect("token spans must reduce");
    let list = SyntaxObject::list(
        vec![
            SyntaxObject::symbol("add", root_span(0, 1, 4), ScopeSet::new(), Phase::Runtime)
                .expect("valid identifier via aggregate path"),
            SyntaxObject::int(1, root_span(0, 5, 6), ScopeSet::new(), Phase::Runtime),
            SyntaxObject::int(2, root_span(0, 7, 8), ScopeSet::new(), Phase::Runtime),
        ],
        list_span,
        ScopeSet::new(),
        Phase::Runtime,
    );

    assert_eq!(list.datum.kind_name(), "list");
    assert_eq!(list.datum.to_string(), "(add 1 2)");
    assert_eq!((list.span.start.0, list.span.end.0), (0, 9));

    let mut order = Vec::new();
    list.visit_spans(&mut |s| order.push((s.start.0, s.end.0)));
    assert_eq!(order, vec![(0, 9), (1, 4), (5, 6), (7, 8)]);

    assert_eq!(WORKSPACE_VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn expander_shaped_scope_flip_and_span_reduction() {
    // Positive + cross-stage consumption: an expander-shaped consumer
    // flips a definition scope across a template tree (deep, functional)
    // and folds every node span down to one covering span via karst-span
    // `join` — the reductions the 0.4 expander will perform. The hygiene
    // union (definition ∪ use) and the subset visibility query follow the
    // §9.38 scope-coloring sketch.
    let template = SyntaxObject::list(
        vec![
            SyntaxObject::symbol("f", root_span(0, 1, 2), ScopeSet::new(), Phase::Runtime).unwrap(),
            SyntaxObject::symbol("x", root_span(0, 3, 4), ScopeSet::new(), Phase::Runtime).unwrap(),
        ],
        root_span(0, 0, 6),
        ScopeSet::new(),
        Phase::Runtime,
    );
    let flipped = template.with_scope_added(ScopeId(4));
    assert!(!template.scopes.contains(ScopeId(4)));
    assert!(flipped.scopes.contains(ScopeId(4)));

    let mut spans = Vec::new();
    flipped.visit_spans(&mut |s| spans.push(s));
    let covering = spans[1..]
        .iter()
        .try_fold(spans[0], |acc, s| acc.join(s))
        .expect("same-file same-generation node spans must reduce");
    assert_eq!((covering.start.0, covering.end.0), (0, 6));

    // Hygiene sets: definition {0,1} ∪ use {1,2} = {0,1,2}; a binding
    // introduced under {0,1} is visible from a reference at {0,1,2}.
    let mut definition = ScopeSet::new();
    definition.insert(ScopeId(0));
    definition.insert(ScopeId(1));
    let mut use_site = ScopeSet::new();
    use_site.insert(ScopeId(1));
    use_site.insert(ScopeId(2));
    let expanded = definition.union(&use_site);
    assert!(definition.is_subset_of(&expanded));
    assert_eq!(expanded.len(), 3);
}

#[test]
fn aggregate_path_rejects_reversed_span_before_stx_construction() {
    // Negative: the no-DUMMY invariant holds in front of syntax
    // construction — a malformed span fails at the karst-span layer
    // first and never reaches a syntax object.
    let err = Span::root(FileId(0), ByteOffset(10), ByteOffset(2))
        .expect_err("reversed span must be rejected before object construction");
    assert!(matches!(err, SpanError::StartAfterEnd { .. }));
}

#[test]
fn aggregate_path_join_rejects_cross_file_stx_spans() {
    // Negative: file identity survives the trip through syntax objects.
    let a = SyntaxObject::symbol("a", root_span(0, 0, 4), ScopeSet::new(), Phase::Runtime).unwrap();
    let b = SyntaxObject::symbol("b", root_span(1, 0, 4), ScopeSet::new(), Phase::Runtime).unwrap();
    let err = a
        .span
        .join(&b.span)
        .expect_err("cross-file span join must be rejected");
    assert!(matches!(err, SpanError::JoinFileMismatch { .. }));
}

#[test]
fn aggregate_path_join_rejects_cross_generation_stx_spans() {
    // Negative: expansion generations survive the trip through syntax
    // objects — the TD-002 boundary (cross-generation join semantics are
    // a macro-phase concern, deliberately rejected at the span layer
    // until the 0.4 adjudication).
    let root = Span::new(FileId(0), ByteOffset(0), ByteOffset(4), ExpansionId::ROOT).unwrap();
    let derived = Span::new(FileId(0), ByteOffset(0), ByteOffset(4), ExpansionId(1)).unwrap();
    let err = root
        .join(&derived)
        .expect_err("cross-generation span join must be rejected");
    assert!(matches!(err, SpanError::JoinExpansionMismatch { .. }));
}

#[test]
fn aggregate_path_rejects_invalid_identifier_text() {
    // Negative: the identifier floor is enforced through the aggregate
    // re-export path (token text that fails the floor never becomes a
    // symbol atom).
    assert!(matches!(
        SyntaxObject::symbol("a b", root_span(0, 0, 3), ScopeSet::new(), Phase::Runtime),
        Err(SyntaxError::InvalidSymbol { .. })
    ));
}

#[test]
fn aggregate_path_rejects_invalid_phase_level() {
    // Negative: phase levels outside the frozen two-level set are
    // rejected through the aggregate re-export path.
    assert_eq!(
        Phase::from_u8(2),
        Err(SyntaxError::InvalidPhase { phase: 2 })
    );
}

#[test]
fn aggregate_path_union_never_mutates_operands() {
    // Negative guard (principle 23): the scope-coloring union produces a
    // new set and never edits its operands — a mutated operand would
    // silently change the hygiene sets of already-built syntax objects.
    let mut a = ScopeSet::new();
    a.insert(ScopeId(0));
    let mut b = ScopeSet::new();
    b.insert(ScopeId(1));
    let merged = a.union(&b);
    assert_eq!(merged.len(), 2);
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    assert!(!a.contains(ScopeId(1)));
    assert!(!b.contains(ScopeId(0)));
}
