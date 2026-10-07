//! Cross-crate integration tests for karst-expander (sub-stage 0.4).

use karst::core::{CoreError, CoreExpr, ModuleItem};
use karst::expander::{ExpandError, expand_program};
use karst::reader::read_program;
use karst::span::FileId;

const FILE: FileId = FileId(0);

fn expand_src(src: &str) -> Result<Vec<ModuleItem>, ExpandError> {
    let forms = read_program(FILE, src).expect("test fixture source must read cleanly");
    expand_program(&forms)
}

// ---------- positive ----------

#[test]
fn fib_source_expands_to_the_compiler_contract() {
    // The Expander → Compiler contract of the 0.4 MUV: fib source →
    // CoreExpr (inside the ModuleItem declaration layer).
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))";
    let items = expand_src(src).expect("fib source must expand");
    assert_eq!(items.len(), 1);
    match &items[0] {
        ModuleItem::Define { name, value, .. } => {
            assert_eq!(name.as_str(), "fib");
            match value {
                CoreExpr::Fn { params, body, .. } => {
                    assert_eq!(params.len(), 1);
                    assert_eq!(params[0].as_str(), "n");
                    // The recursive body: if (< n 2) n (+ (fib …) (fib …))
                    match **body {
                        CoreExpr::If {
                            ref cond,
                            ref then_branch,
                            ref else_branch,
                            ..
                        } => {
                            // (< n 2) — `<` resolved from STAGE0_GLOBALS.
                            match **cond {
                                CoreExpr::Apply {
                                    ref func, ref args, ..
                                } => {
                                    match **func {
                                        CoreExpr::Var { ref name, .. } => {
                                            assert_eq!(name.as_str(), "<")
                                        }
                                        ref other => panic!("expected var func, got {other}"),
                                    }
                                    assert_eq!(args.len(), 2);
                                }
                                ref other => panic!("expected apply cond, got {other}"),
                            }
                            assert!(matches!(**then_branch, CoreExpr::Var { .. }));
                            // (+ (fib (- n 1)) (fib (- n 2))) — the
                            // self-reference resolved through the
                            // letrec-shaped top-level rule.
                            match **else_branch {
                                CoreExpr::Apply {
                                    ref func, ref args, ..
                                } => {
                                    assert!(matches!(**func, CoreExpr::Var { .. }));
                                    assert_eq!(args.len(), 2);
                                    for arg in args {
                                        assert!(matches!(arg, CoreExpr::Apply { .. }));
                                    }
                                }
                                ref other => panic!("expected apply else, got {other}"),
                            }
                        }
                        ref other => panic!("expected if body, got {other}"),
                    }
                }
                other => panic!("expected fn value, got {other}"),
            }
        }
        other => panic!("expected define, got {other}"),
    }
    // Span discipline over the full reader→expander path: every node
    // carries a real source span and all joins succeed (same file and
    // expansion generation — the TD-002 adjudication: desugaring
    // reuses source spans, cross-generation joins never occur).
    let mut spans = Vec::new();
    items[0].visit_spans(&mut |span| spans.push(span));
    assert!(spans.len() >= 12, "fib expansion must carry >= 12 spans");
    spans
        .iter()
        .try_fold(spans[0], |acc, s| acc.join(s))
        .expect("all spans must join (one generation)");
}

#[test]
fn do_source_desugars_to_discard_let_end_to_end() {
    // (define (f x) (do (write-line x) x)) — the body is a Begin →
    // Let chain with one `_` discard and `x` as the body.
    let src = "(define (f x) (do (write-line x) x))";
    let items = expand_src(src).expect("do source must expand");
    match &items[0] {
        ModuleItem::Define { value, .. } => match value {
            CoreExpr::Fn { body, .. } => match **body {
                CoreExpr::Let {
                    ref bindings,
                    ref body,
                    ..
                } => {
                    assert_eq!(bindings.len(), 1);
                    assert_eq!(bindings[0].name.as_str(), "_");
                    // The discarded step is the write-line application
                    // (a Stage 0 global).
                    match &bindings[0].value {
                        CoreExpr::Apply { func, args, .. } => {
                            assert!(matches!(**func, CoreExpr::Var { .. }));
                            assert_eq!(args.len(), 1);
                            assert!(matches!(args[0], CoreExpr::Var { .. }));
                        }
                        other => panic!("expected apply step, got {other}"),
                    }
                    assert!(matches!(**body, CoreExpr::Var { .. }));
                }
                ref other => panic!("expected let body, got {other}"),
            },
            other => panic!("expected fn value, got {other}"),
        },
        other => panic!("expected define, got {other}"),
    }
}

// ---------- negative ----------

#[test]
fn unbound_identifier_from_source_fails_with_bytes() {
    let src = "(define (f) undefined-thing)";
    let err = expand_src(src).expect_err("unbound value must fail");
    match err {
        ExpandError::UnboundIdentifier { name, span } => {
            assert_eq!(name, "undefined-thing");
            // Byte-precise location of the offending atom: `(`=0,
            // `define`=1..7, `(f)`=8..11, atom=12..27.
            assert_eq!(span.start.0, 12);
            assert_eq!(span.end.0, 27);
        }
        other => panic!("expected unbound identifier, got {other}"),
    }
}

#[test]
fn bare_top_level_expression_from_source_fails() {
    let src = "(fib 10)";
    let err = expand_src(src).expect_err("bare expression must fail");
    assert!(matches!(err, ExpandError::TopLevelExpression { .. }));
}

#[test]
fn set_to_unbound_target_from_source_fails() {
    let src = "(define (f) (set! nope 1))";
    let err = expand_src(src).expect_err("unbound set! target must fail");
    match err {
        ExpandError::UnboundIdentifier { name, .. } => assert_eq!(name, "nope"),
        other => panic!("expected unbound identifier, got {other}"),
    }
}

#[test]
fn duplicate_params_from_source_reassert_via_core() {
    let src = "(define (f x x) x)";
    let err = expand_src(src).expect_err("duplicate params must fail");
    match err {
        ExpandError::Core { source, .. } => {
            assert_eq!(
                source,
                CoreError::DuplicateParam {
                    name: "x".to_owned()
                }
            );
        }
        other => panic!("expected core error, got {other}"),
    }
}

#[test]
fn malformed_if_from_source_fails() {
    let src = "(define (f) (if 1 2))";
    let err = expand_src(src).expect_err("two-armed if must fail");
    match err {
        ExpandError::MalformedForm { form, .. } => assert_eq!(form, "if"),
        other => panic!("expected malformed if, got {other}"),
    }
}

#[test]
fn nested_module_is_scope_isolated_from_outer_defines() {
    // Module isolation (docs/stage0.md §8.9 direction): `a` defined at
    // the outer top level is invisible inside the nested module.
    let src = "(define a 1) (module m (define (g) a))";
    let err = expand_src(src).expect_err("outer define must be invisible in module");
    match err {
        ExpandError::UnboundIdentifier { name, span } => {
            assert_eq!(name, "a");
            // The `a` reference sits at byte 35 (after `(define a 1) `
            // = 13 bytes and `(module m (define (g) `).
            assert_eq!(span.start.0, 35);
        }
        other => panic!("expected unbound identifier, got {other}"),
    }
}
