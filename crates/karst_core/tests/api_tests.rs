//! karst-core crate-level integration tests: verify the public API surface

use karst_core::{Binding, CoreError, CoreExpr, HandlerClause, LiteralValue, ModuleItem};
use karst_span::{ByteOffset, ExpansionId, FileId, Span};
use karst_syntax::Symbol;

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
    // Compile-level surface check: every documented name is reachable from
    // the crate root (SOP §10.1 rule 4 explicit re-export list).
    let symbol = Symbol::new("x").expect("valid symbol via public path");
    assert_eq!(symbol.as_str(), "x");

    let literal = CoreExpr::Literal {
        span: span(0, 2),
        value: LiteralValue::Int(42),
    };
    assert_eq!(literal.kind_name(), "literal");
    let _binding: Binding = Binding::new(span(0, 1), "x", literal.clone()).unwrap();
    let _clause: HandlerClause = HandlerClause::new(span(0, 1), "read", literal.clone()).unwrap();
    let _err: CoreError = CoreError::InvalidSymbol {
        text: String::new(),
    };
}

#[test]
fn validated_constructors_build_full_program() {
    // A representative program: define f = fn(x) => if x then 1 else 2,
    // handled under an effect clause — every name-bearing node goes
    // through a validated constructor.
    let body = CoreExpr::If {
        span: span(10, 30),
        cond: Box::new(CoreExpr::var(span(10, 11), "x").unwrap()),
        then_branch: Box::new(CoreExpr::Literal {
            span: span(16, 17),
            value: LiteralValue::Int(1),
        }),
        else_branch: Box::new(CoreExpr::Literal {
            span: span(24, 25),
            value: LiteralValue::Int(2),
        }),
    };
    let function = CoreExpr::fn_(span(0, 30), &["x"], body).unwrap();
    let handled = CoreExpr::handle(
        span(0, 60),
        function,
        vec![
            HandlerClause::new(
                span(40, 50),
                "read",
                CoreExpr::var(span(41, 42), "h").unwrap(),
            )
            .unwrap(),
        ],
    )
    .unwrap();
    let item = ModuleItem::define(span(0, 60), "f", handled).unwrap();

    let mut span_count = 0usize;
    item.visit_spans(&mut |_s| span_count += 1);
    assert!(
        span_count >= 6,
        "nested program must report >= 6 spans, got {span_count}"
    );
    assert_eq!(item.kind_name(), "define");
}

#[test]
fn public_fn_constructor_rejects_duplicate_params() {
    let err = CoreExpr::fn_(
        span(0, 1),
        &["x", "y", "x"],
        CoreExpr::Literal {
            span: span(0, 1),
            value: LiteralValue::Nil,
        },
    )
    .expect_err("duplicate params must be rejected via public path");
    assert_eq!(
        err,
        CoreError::DuplicateParam {
            name: "x".to_owned()
        }
    );
}

#[test]
fn public_handle_constructor_rejects_duplicate_effects() {
    let literal = || CoreExpr::Literal {
        span: span(0, 1),
        value: LiteralValue::Nil,
    };
    let err = CoreExpr::handle(
        span(0, 1),
        literal(),
        vec![
            HandlerClause::new(span(0, 1), "read", literal()).unwrap(),
            HandlerClause::new(span(0, 1), "read", literal()).unwrap(),
        ],
    )
    .expect_err("duplicate effect clauses must be rejected via public path");
    assert_eq!(
        err,
        CoreError::DuplicateHandler {
            effect: "read".to_owned()
        }
    );
}

#[test]
fn public_var_constructor_rejects_invalid_name() {
    assert!(matches!(
        CoreExpr::var(span(0, 1), "a b"),
        Err(CoreError::InvalidSymbol { .. })
    ));
}

#[test]
fn public_define_rejects_invalid_name() {
    let value = CoreExpr::Literal {
        span: span(0, 1),
        value: LiteralValue::Nil,
    };
    assert!(matches!(
        ModuleItem::define(span(0, 1), "a b", value),
        Err(CoreError::InvalidSymbol { .. })
    ));
}

#[test]
fn public_require_rejects_invalid_module_name() {
    assert!(matches!(
        ModuleItem::require(span(0, 1), "\t"),
        Err(CoreError::InvalidSymbol { .. })
    ));
}

#[test]
fn public_module_rejects_invalid_name() {
    assert!(matches!(
        ModuleItem::module(span(0, 1), "a b", vec![]),
        Err(CoreError::InvalidSymbol { .. })
    ));
}
