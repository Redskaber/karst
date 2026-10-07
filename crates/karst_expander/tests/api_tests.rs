//! Crate-level API tests for karst-expander

use karst_core::{CoreExpr, LiteralValue, ModuleItem};
use karst_expander::{ExpandCtx, ExpandError, STAGE0_GLOBALS, expand, expand_program};
use karst_span::{ByteOffset, ExpansionId, FileId, Span};
use karst_syntax::{Phase, ScopeSet, SyntaxObject};

fn sp(start: u32, end: u32) -> Span {
    Span::new(
        FileId(0),
        ByteOffset(start),
        ByteOffset(end),
        ExpansionId::ROOT,
    )
    .expect("test fixture span must be well-formed")
}

fn sym(start: u32, end: u32, text: &str) -> SyntaxObject {
    SyntaxObject::symbol(text, sp(start, end), ScopeSet::new(), Phase::Runtime)
        .expect("test fixture symbol must be valid")
}

fn int(start: u32, end: u32, value: i64) -> SyntaxObject {
    SyntaxObject::int(value, sp(start, end), ScopeSet::new(), Phase::Runtime)
}

fn lst(start: u32, end: u32, items: Vec<SyntaxObject>) -> SyntaxObject {
    SyntaxObject::list(items, sp(start, end), ScopeSet::new(), Phase::Runtime)
}

/// (define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))
fn fib_define_form() -> SyntaxObject {
    let fib_at = |s: u32, e: u32| sym(s, e, "fib");
    lst(
        0,
        60,
        vec![
            sym(0, 6, "define"),
            lst(8, 15, vec![fib_at(9, 12), sym(13, 14, "n")]),
            lst(
                17,
                59,
                vec![
                    sym(18, 20, "if"),
                    lst(
                        22,
                        28,
                        vec![sym(23, 24, "<"), sym(25, 26, "n"), int(27, 28, 2)],
                    ),
                    sym(30, 31, "n"),
                    lst(
                        33,
                        58,
                        vec![
                            sym(34, 35, "+"),
                            lst(
                                37,
                                46,
                                vec![
                                    fib_at(38, 41),
                                    lst(
                                        43,
                                        45,
                                        vec![sym(44, 45, "-"), sym(44, 45, "n"), int(44, 45, 1)],
                                    ),
                                ],
                            ),
                            lst(
                                49,
                                57,
                                vec![
                                    fib_at(50, 53),
                                    lst(
                                        54,
                                        56,
                                        vec![sym(55, 56, "-"), sym(55, 56, "n"), int(55, 56, 2)],
                                    ),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
        ],
    )
}

// ---------- positive ----------

#[test]
fn expand_program_delivers_the_fib_contract() {
    let items = expand_program(&[fib_define_form()]).expect("fib define must expand");
    assert_eq!(items.len(), 1);
    match &items[0] {
        ModuleItem::Define { name, value, .. } => {
            assert_eq!(name.as_str(), "fib");
            match value {
                CoreExpr::Fn { params, body, .. } => {
                    assert_eq!(params.len(), 1);
                    assert_eq!(params[0].as_str(), "n");
                    // The body is the recursive if/apply tree.
                    assert!(matches!(**body, CoreExpr::If { .. }));
                }
                other => panic!("expected fn value, got {other}"),
            }
        }
        other => panic!("expected define, got {other}"),
    }
    // TD-002 discipline: every produced span joins cleanly — all same
    // file and expansion generation (desugared nodes reuse source
    // spans; no cross-generation joins ever occur in this layer).
    let mut spans = Vec::new();
    items[0].visit_spans(&mut |span| spans.push(span));
    assert!(spans.len() >= 8, "fib tree must carry >= 8 spans");
    let joined = spans
        .iter()
        .try_fold(spans[0], |acc, s| acc.join(s))
        .expect("all fib spans must join (same generation)");
    assert_eq!(joined, sp(0, 60));
}

#[test]
fn stage0_globals_resolve_as_flat_fallback() {
    // The documented surface: 18 names, arithmetic/pair/IO.
    assert_eq!(STAGE0_GLOBALS.len(), 18);
    for name in [
        "+",
        "-",
        "*",
        "/",
        "<",
        "=",
        "car",
        "cons",
        "nil?",
        "not",
        "read-line",
        "write-line",
    ] {
        assert!(STAGE0_GLOBALS.contains(&name), "`{name}` must be listed");
    }
    // (+ (car nil) 1) — `+` and `car` resolve as globals, `nil` is the
    // literal keyword.
    let mut ctx = ExpandCtx::new();
    let form = lst(
        0,
        15,
        vec![
            sym(1, 2, "+"),
            lst(4, 13, vec![sym(5, 8, "car"), sym(10, 13, "nil")]),
            int(14, 15, 1),
        ],
    );
    let expr = expand(&mut ctx, &form).expect("globals form must expand");
    match expr {
        CoreExpr::Apply { func, args, .. } => {
            assert_eq!(*func, CoreExpr::var(sp(1, 2), "+").unwrap());
            assert_eq!(args.len(), 2);
            assert!(matches!(args[0], CoreExpr::Apply { .. }));
            assert_eq!(
                args[1],
                CoreExpr::Literal {
                    span: sp(14, 15),
                    value: LiteralValue::Int(1)
                }
            );
        }
        other => panic!("expected apply, got {other}"),
    }
}

// ---------- negative ----------

#[test]
fn empty_do_form_is_malformed() {
    // Through the expression entry: at module top level the same form
    // would be a bare expression first (declarations only).
    let mut ctx = ExpandCtx::new();
    let form = lst(0, 3, vec![sym(0, 2, "do")]);
    let err = expand(&mut ctx, &form).expect_err("empty do must fail");
    match err {
        ExpandError::MalformedForm { form, .. } => assert_eq!(form, "do"),
        other => panic!("expected malformed do, got {other}"),
    }
}

#[test]
fn set_to_string_target_is_malformed() {
    let mut ctx = ExpandCtx::new();
    let form = lst(
        0,
        13,
        vec![
            sym(0, 4, "set!"),
            SyntaxObject::string("s".to_owned(), sp(5, 7), ScopeSet::new(), Phase::Runtime),
            int(8, 9, 1),
        ],
    );
    let err = expand(&mut ctx, &form).expect_err("string target must fail");
    match err {
        ExpandError::MalformedForm { form, .. } => assert_eq!(form, "set!"),
        other => panic!("expected malformed set!, got {other}"),
    }
}

#[test]
fn require_with_symbol_is_malformed() {
    let form = lst(0, 12, vec![sym(0, 7, "require"), sym(9, 10, "m")]);
    let err = expand_program(&[form]).expect_err("symbol require must fail");
    match err {
        ExpandError::MalformedForm { form, .. } => assert_eq!(form, "require"),
        other => panic!("expected malformed require, got {other}"),
    }
}

#[test]
fn define_with_bad_name_shape_is_malformed() {
    // (define 123 1) — the name position is not a symbol or a
    // (name params) list.
    let form = lst(
        0,
        13,
        vec![sym(0, 6, "define"), int(8, 11, 123), int(12, 13, 1)],
    );
    let err = expand_program(&[form]).expect_err("numeric define name must fail");
    match err {
        ExpandError::MalformedForm { form, .. } => assert_eq!(form, "define"),
        other => panic!("expected malformed define, got {other}"),
    }
}

#[test]
fn perform_with_non_symbol_effect_is_malformed() {
    let mut ctx = ExpandCtx::new();
    let form = lst(0, 11, vec![sym(0, 7, "perform"), int(9, 10, 1)]);
    let err = expand(&mut ctx, &form).expect_err("numeric effect must fail");
    match err {
        ExpandError::MalformedForm { form, .. } => assert_eq!(form, "perform"),
        other => panic!("expected malformed perform, got {other}"),
    }
}

#[test]
fn let_with_mixed_binding_spec_is_malformed() {
    // (let ((x 1) y) 2) — first spec item is a list, so the multi
    // path requires every item to be a (name value) pair.
    let mut ctx = ExpandCtx::new();
    let form = lst(
        0,
        17,
        vec![
            sym(0, 3, "let"),
            lst(
                5,
                13,
                vec![
                    lst(6, 10, vec![sym(7, 8, "x"), int(9, 10, 1)]),
                    sym(12, 13, "y"),
                ],
            ),
            int(15, 16, 2),
        ],
    );
    let err = expand(&mut ctx, &form).expect_err("mixed spec must fail");
    match err {
        ExpandError::MalformedForm { form, .. } => assert_eq!(form, "let"),
        other => panic!("expected malformed let, got {other}"),
    }
}
