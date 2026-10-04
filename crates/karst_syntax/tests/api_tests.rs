//! karst-syntax crate-level integration tests: verify the public API

use karst_span::{ByteOffset, ExpansionId, FileId, Span};
use karst_syntax::{Phase, ScopeId, ScopeSet, Symbol, SyntaxError, SyntaxObject};

fn span(start: u32, end: u32) -> Span {
    Span::new(
        FileId(0),
        ByteOffset(start),
        ByteOffset(end),
        ExpansionId::ROOT,
    )
    .expect("test fixture span must be well-formed")
}

#[test]
fn public_api_exports_documented_surface() {
    // Compile-level surface check: every documented name is reachable
    let symbol = Symbol::new("x").expect("valid symbol via public path");
    assert_eq!(symbol.as_str(), "x");

    let mut scopes = ScopeSet::new();
    assert!(scopes.insert(ScopeId(1)));
    assert!(scopes.contains(ScopeId(1)));

    let phase = Phase::from_u8(1).expect("level 1 decodes via public path");
    assert_eq!(phase, Phase::ExpandTime);

    let object = SyntaxObject::symbol("x", span(0, 1), scopes, phase)
        .expect("checked constructor via public path");
    assert_eq!(object.datum.kind_name(), "symbol");
    let _err: SyntaxError = SyntaxError::InvalidPhase { phase: 2 };
}

#[test]
fn validated_constructors_build_reader_shaped_program() {
    // A reader-shaped program fragment `(f x 42)` — every node goes
    // through a public constructor, spans are distinct, and the
    // pre-order traversal covers the whole tree.
    let head = SyntaxObject::symbol("f", span(1, 2), ScopeSet::new(), Phase::Runtime).unwrap();
    let arg = SyntaxObject::symbol("x", span(3, 4), ScopeSet::new(), Phase::Runtime).unwrap();
    let lit = SyntaxObject::int(42, span(5, 7), ScopeSet::new(), Phase::Runtime);
    let list = SyntaxObject::list(
        vec![head, arg, lit],
        span(0, 8),
        ScopeSet::new(),
        Phase::Runtime,
    );

    assert_eq!(list.datum.to_string(), "(f x 42)");
    let mut count = 0usize;
    list.visit_spans(&mut |_s| count += 1);
    assert_eq!(count, 4, "list + three children");
    // Deep scope flip through the public path.
    let flipped = list.with_scope_added(ScopeId(3));
    assert!(flipped.scopes.contains(ScopeId(3)));
    assert!(!list.scopes.contains(ScopeId(3)));
}

#[test]
fn public_symbol_constructor_rejects_empty_text() {
    assert!(matches!(
        SyntaxObject::symbol("", span(0, 1), ScopeSet::new(), Phase::Runtime),
        Err(SyntaxError::InvalidSymbol { .. })
    ));
}

#[test]
fn public_symbol_constructor_rejects_whitespace() {
    assert!(matches!(
        SyntaxObject::symbol("a b", span(0, 1), ScopeSet::new(), Phase::Runtime),
        Err(SyntaxError::InvalidSymbol { .. })
    ));
}

#[test]
fn public_symbol_constructor_rejects_control_char() {
    assert!(matches!(
        SyntaxObject::symbol("a\u{0}b", span(0, 1), ScopeSet::new(), Phase::Runtime),
        Err(SyntaxError::InvalidSymbol { .. })
    ));
}

#[test]
fn public_phase_decoder_rejects_out_of_range() {
    assert_eq!(
        Phase::from_u8(2),
        Err(SyntaxError::InvalidPhase { phase: 2 })
    );
}

#[test]
fn public_scope_queries_return_false_for_missing() {
    let mut set = ScopeSet::new();
    set.insert(ScopeId(1));
    assert!(!set.contains(ScopeId(0)));
    assert!(!set.insert(ScopeId(1)));
    // {1} is not a subset of the empty set (the empty set itself *is* a
    // subset of everything — asserted the other way round on purpose).
    assert!(!set.is_subset_of(&ScopeSet::new()));
    assert!(ScopeSet::new().is_subset_of(&set));
}

#[test]
fn public_union_does_not_mutate_operands() {
    let mut a = ScopeSet::new();
    a.insert(ScopeId(0));
    let mut b = ScopeSet::new();
    b.insert(ScopeId(1));
    let merged = a.union(&b);
    assert_eq!(merged.len(), 2);
    assert_eq!(a.len(), 1, "left operand untouched (principle 23)");
    assert_eq!(b.len(), 1, "right operand untouched (principle 23)");
}
