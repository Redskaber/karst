//! Stage 0.1 plan tests — karst-core CoreExpr seed MUV, cross-crate integration level.
// Shared helpers: mounted once by the runner entry (tests/runner.rs) and
// imported here via `super::`.
use super::common::{close_token, covering_span, ident_token, int_token, open_token, root_span};
use karst::WORKSPACE_VERSION;
use karst::core::{CoreExpr, HandlerClause, LiteralValue, ModuleItem};
use karst::span::{ByteOffset, ExpansionId, FileId, Span, SpanError};

#[test]
fn expander_shaped_program_span_reduction() {
    // Positive + cross-stage consumption: an expander-shaped consumer
    // builds a declaration-layer program (define f = fn(x) => ...) and
    // folds every node span down to one covering span via karst-span
    // `join` — the reduction the 0.4 expander will perform.
    let body = CoreExpr::If {
        span: root_span(0, 12, 30),
        cond: Box::new(CoreExpr::var(root_span(0, 12, 13), "x").unwrap()),
        then_branch: Box::new(CoreExpr::Literal {
            span: root_span(0, 18, 19),
            value: LiteralValue::Int(1),
        }),
        else_branch: Box::new(CoreExpr::Literal {
            span: root_span(0, 24, 25),
            value: LiteralValue::Int(2),
        }),
    };
    let function = CoreExpr::fn_(root_span(0, 0, 30), &["x"], body)
        .expect("valid fn must construct via aggregate path");
    let item = ModuleItem::define(root_span(0, 0, 40), "f", function)
        .expect("valid define must construct via aggregate path");

    let mut spans = Vec::new();
    item.visit_spans(&mut |s| spans.push(s));
    assert_eq!(spans.len(), 6, "define + fn + if + cond + then + else");

    let covering = spans[1..]
        .iter()
        .try_fold(spans[0], |acc, s| acc.join(s))
        .expect("same-file same-generation node spans must reduce");
    assert_eq!((covering.start.0, covering.end.0), (0, 40));
    assert_eq!(WORKSPACE_VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn reader_shaped_token_to_node_pattern() {
    // Positive + cross-stage consumption: a reader-shaped consumer maps
    // real reader tokens to core nodes, then folds the
    // node spans — the contract Stage 0.3 will rely on (mirrors the 0.0
    // span seed reduction pattern).
    let tokens = [
        open_token(root_span(0, 0, 1)),
        ident_token("f", root_span(0, 1, 2)),
        int_token(42, root_span(0, 3, 5)),
        close_token(root_span(0, 6, 7)),
    ];
    let node = CoreExpr::Apply {
        span: covering_span(&tokens).expect("token spans must reduce"),
        func: Box::new(CoreExpr::var(root_span(0, 1, 2), "f").unwrap()),
        args: vec![CoreExpr::Literal {
            span: root_span(0, 3, 5),
            value: LiteralValue::Int(42),
        }],
    };
    assert_eq!(node.kind_name(), "apply");
    assert_eq!((node.span().start.0, node.span().end.0), (0, 7));

    let mut spans = Vec::new();
    node.visit_spans(&mut |s| spans.push(s));
    let folded = covering_span(&[
        ident_token("node", spans[0]),
        ident_token("func", spans[1]),
        ident_token("arg", spans[2]),
    ])
    .expect("node spans must reduce");
    assert_eq!((folded.start.0, folded.end.0), (0, 7));
}

#[test]
fn aggregate_path_rejects_reversed_span_before_node_construction() {
    // Negative: the span invariant holds in front of core construction —
    // a malformed span fails at the karst-span layer first and never
    // reaches a node.
    let err = Span::root(FileId(0), ByteOffset(10), ByteOffset(2))
        .expect_err("reversed span must be rejected before node construction");
    assert!(matches!(err, SpanError::StartAfterEnd { .. }));
}

#[test]
fn aggregate_path_join_rejects_cross_file_node_spans() {
    // Negative: file identity survives the trip through core nodes.
    let a = CoreExpr::var(root_span(0, 0, 4), "a").unwrap();
    let b = CoreExpr::var(root_span(1, 0, 4), "b").unwrap();
    let err = a
        .span()
        .join(&b.span())
        .expect_err("cross-file node spans must not join");
    assert_eq!(
        err,
        SpanError::JoinFileMismatch {
            lhs: FileId(0),
            rhs: FileId(1)
        }
    );
}

#[test]
fn aggregate_path_join_rejects_cross_generation_node_spans() {
    // Negative: the TD-002 expansion-generation boundary stays visible to
    // core consumers (macro-phase join semantics remain undefined).
    let root_span_gen0 =
        Span::new(FileId(0), ByteOffset(0), ByteOffset(4), ExpansionId(0)).unwrap();
    let expanded = Span::new(FileId(0), ByteOffset(8), ByteOffset(12), ExpansionId(2)).unwrap();
    let a = CoreExpr::Literal {
        span: root_span_gen0,
        value: LiteralValue::Nil,
    };
    let b = CoreExpr::Literal {
        span: expanded,
        value: LiteralValue::Nil,
    };
    assert!(matches!(
        a.span().join(&b.span()),
        Err(SpanError::JoinExpansionMismatch { .. })
    ));
}

#[test]
fn aggregate_path_fn_rejects_duplicate_params() {
    let err = CoreExpr::fn_(
        root_span(0, 0, 10),
        &["x", "x"],
        CoreExpr::Literal {
            span: root_span(0, 0, 1),
            value: LiteralValue::Nil,
        },
    )
    .expect_err("duplicate params must be rejected via aggregate path");
    assert_eq!(
        err,
        karst::core::CoreError::DuplicateParam {
            name: "x".to_owned()
        }
    );
}

#[test]
fn aggregate_path_rejects_invalid_symbol() {
    let err = CoreExpr::var(root_span(0, 0, 1), "a b")
        .expect_err("invalid symbol must be rejected via aggregate path");
    let rendered = err.to_string();
    assert!(rendered.contains("`a b`"));
    assert!(!rendered.ends_with('.'));
}

#[test]
fn aggregate_path_handle_rejects_duplicate_handlers() {
    let literal = || CoreExpr::Literal {
        span: root_span(0, 0, 1),
        value: LiteralValue::Nil,
    };
    let err = CoreExpr::handle(
        root_span(0, 0, 10),
        literal(),
        vec![
            HandlerClause::new(root_span(0, 0, 2), "read", literal()).unwrap(),
            HandlerClause::new(root_span(0, 3, 5), "read", literal()).unwrap(),
        ],
    )
    .expect_err("duplicate handler clauses must be rejected via aggregate path");
    assert_eq!(
        err,
        karst::core::CoreError::DuplicateHandler {
            effect: "read".to_owned()
        }
    );
}
