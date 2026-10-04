//! @path: karst/crates/karst_core/expr.rs
//! @author: redskaber
//! @datetime: 2026-10-01
//! @discription: karst::crates::karst_core::expr
//!
//! CoreExpr: the frozen semantic primitive expression ADT

use std::collections::HashSet;
use std::fmt;

use crate::error::{CoreError, from_syntax_symbol};

use karst_span::span::Span;
use karst_syntax::symbol::Symbol;

/// Literal value
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Pair(Box<LiteralValue>, Box<LiteralValue>),
    Nil,
}

impl LiteralValue {
    pub fn kind_name(&self) -> &'static str {
        match self {
            LiteralValue::Int(_) => "int",
            LiteralValue::Float(_) => "float",
            LiteralValue::Bool(_) => "bool",
            LiteralValue::String(_) => "string",
            LiteralValue::Pair(..) => "pair",
            LiteralValue::Nil => "nil",
        }
    }
}

impl fmt::Display for LiteralValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LiteralValue::Int(v) => write!(f, "{v}"),
            LiteralValue::Float(v) => write!(f, "{v:?}"),
            LiteralValue::Bool(v) => write!(f, "{v}"),
            LiteralValue::String(v) => write!(f, "{v:?}"),
            LiteralValue::Pair(car, cdr) => write!(f, "({car} . {cdr})"),
            LiteralValue::Nil => f.write_str("nil"),
        }
    }
}

/// One `let` binding site: `name` bound to `value`, covering `span`
#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub span: Span,
    pub name: Symbol,
    pub value: CoreExpr,
}

impl Binding {
    pub fn new(span: Span, name: &str, value: CoreExpr) -> Result<Binding, CoreError> {
        Ok(Binding {
            span,
            name: from_syntax_symbol(Symbol::new(name))?,
            value,
        })
    }
}

/// One `handle` clause: `handler` installed for `effect`, at `span`
#[derive(Debug, Clone, PartialEq)]
pub struct HandlerClause {
    pub span: Span,
    pub effect: Symbol,
    pub handler: Box<CoreExpr>,
}

impl HandlerClause {
    pub fn new(span: Span, effect: &str, handler: CoreExpr) -> Result<HandlerClause, CoreError> {
        Ok(HandlerClause {
            span,
            effect: from_syntax_symbol(Symbol::new(effect))?,
            handler: Box::new(handler),
        })
    }
}

/// Core-expression ADT (S-experession design base core expr)
#[derive(Debug, Clone, PartialEq)]
pub enum CoreExpr {
    /// Function abstraction
    /// `(Fn {params, body}, env) -> closure`
    Fn {
        span: Span,
        params: Vec<Symbol>,
        body: Box<CoreExpr>,
    },
    /// Application
    /// `(Apply {func, args}, env) -> apply(eval(func), eval(args))`
    Apply {
        span: Span,
        func: Box<CoreExpr>,
        args: Vec<CoreExpr>,
    },
    /// Conditional branch
    If {
        span: Span,
        cond: Box<CoreExpr>,
        then_branch: Box<CoreExpr>,
        else_branch: Box<CoreExpr>,
    },
    /// Variable reference
    Var { span: Span, name: Symbol },
    /// Literal value
    Literal { span: Span, value: LiteralValue },
    /// Assign `==`
    Assign {
        span: Span,
        name: Symbol,
        value: Box<CoreExpr>,
    },
    /// Seq binding from `(let* semantics;)` `do`
    Let {
        span: Span,
        bindings: Vec<Binding>,
        body: Box<CoreExpr>,
    },
    /// Effect performance
    Perform {
        span: Span,
        effect: Symbol,
        args: Vec<CoreExpr>,
    },
    /// Effect handing
    Handle {
        span: Span,
        bpdy: Box<CoreExpr>,
        handlers: Vec<HandlerClause>,
    },
}

impl CoreExpr {
    pub fn span(&self) -> Span {
        match self {
            CoreExpr::Fn { span, .. }
            | CoreExpr::Apply { span, .. }
            | CoreExpr::If { span, .. }
            | CoreExpr::Var { span, .. }
            | CoreExpr::Literal { span, .. }
            | CoreExpr::Assign { span, .. }
            | CoreExpr::Let { span, .. }
            | CoreExpr::Perform { span, .. }
            | CoreExpr::Handle { span, .. } => *span,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            CoreExpr::Fn { .. } => "fn",
            CoreExpr::Apply { .. } => "apply",
            CoreExpr::If { .. } => "if",
            CoreExpr::Var { .. } => "var",
            CoreExpr::Literal { .. } => "literal",
            CoreExpr::Assign { .. } => "assign",
            CoreExpr::Let { .. } => "let",
            CoreExpr::Perform { .. } => "perform",
            CoreExpr::Handle { .. } => "handle",
        }
    }

    /// Pre-order span traversal
    pub fn visit_spans(&self, f: &mut impl FnMut(Span)) {
        match self {
            CoreExpr::Fn { span, body, .. } => {
                f(*span);
                body.visit_spans(f);
            }
            CoreExpr::Apply { span, func, args } => {
                f(*span);
                func.visit_spans(f);
                for arg in args {
                    arg.visit_spans(f);
                }
            }
            CoreExpr::If {
                span,
                cond,
                then_branch,
                else_branch,
            } => {
                f(*span);
                cond.visit_spans(f);
                then_branch.visit_spans(f);
                else_branch.visit_spans(f);
            }
            CoreExpr::Var { span, .. } => f(*span),
            CoreExpr::Literal { span, .. } => f(*span),
            CoreExpr::Assign { span, value, .. } => {
                f(*span);
                value.visit_spans(f);
            }
            CoreExpr::Let {
                span,
                bindings,
                body,
            } => {
                f(*span);
                for binding in bindings {
                    f(binding.span);
                    binding.value.visit_spans(f);
                }
                body.visit_spans(f);
            }
            CoreExpr::Perform { span, args, .. } => {
                f(*span);
                for arg in args {
                    arg.visit_spans(f);
                }
            }
            CoreExpr::Handle {
                span,
                bpdy,
                handlers,
            } => {
                f(*span);
                bpdy.visit_spans(f);
                for handle in handlers {
                    f(handle.span);
                    handle.handler.visit_spans(f);
                }
            }
        }
    }

    /// Checked `fn` constructor from source-shaped parameter text
    pub fn fn_(span: Span, params: &[&str], body: CoreExpr) -> Result<CoreExpr, CoreError> {
        let symbols = params
            .iter()
            .map(|raw| from_syntax_symbol(Symbol::new(raw)))
            .collect::<Result<_, _>>()?;
        let mut seen: HashSet<&str> = HashSet::with_capacity(params.len());
        for raw in params {
            if !seen.insert(*raw) {
                return Err(CoreError::DuplicateParam {
                    name: (*raw).to_owned(),
                });
            }
        }
        Ok(CoreExpr::Fn {
            span,
            params: symbols,
            body: Box::new(body),
        })
    }

    pub fn var(span: Span, name: &str) -> Result<CoreExpr, CoreError> {
        Ok(CoreExpr::Var {
            span,
            name: from_syntax_symbol(Symbol::new(name))?,
        })
    }

    pub fn assign(span: Span, name: &str, value: CoreExpr) -> Result<CoreExpr, CoreError> {
        Ok(CoreExpr::Assign {
            span,
            name: from_syntax_symbol(Symbol::new(name))?,
            value: Box::new(value),
        })
    }

    pub fn perform(span: Span, effect: &str, args: Vec<CoreExpr>) -> Result<CoreExpr, CoreError> {
        Ok(CoreExpr::Perform {
            span,
            effect: from_syntax_symbol(Symbol::new(effect))?,
            args,
        })
    }

    /// Checked `handle` constructor over pre-built clauses
    pub fn handle(
        span: Span,
        body: CoreExpr,
        handlers: Vec<HandlerClause>,
    ) -> Result<CoreExpr, CoreError> {
        let mut seen = HashSet::with_capacity(handlers.len());
        for handle in &handlers {
            if !seen.insert(handle.effect.as_str()) {
                return Err(CoreError::DuplicateHandler {
                    effect: handle.effect.as_str().to_owned(),
                });
            }
        }
        Ok(CoreExpr::Handle {
            span,
            bpdy: Box::new(body),
            handlers,
        })
    }
}

impl fmt::Display for CoreExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.kind_name())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the CoreExpr / LiteralValue feature points (sub-stage test doc FP2-FP4).

    use super::*;
    use karst_span::{ByteOffset, ExpansionId, FileId};

    fn span(start: u32, end: u32) -> Span {
        Span::new(
            FileId(0),
            ByteOffset(start),
            ByteOffset(end),
            ExpansionId::ROOT,
        )
        .expect("test fixture span must be well-formed")
    }

    fn sample_literal() -> CoreExpr {
        CoreExpr::Literal {
            span: span(30, 32),
            value: LiteralValue::Int(42),
        }
    }

    // ---------- positive ----------

    #[test]
    fn kind_name_covers_all_nine_variants() {
        let body = sample_literal();
        let cases: Vec<(CoreExpr, &'static str)> = vec![
            (CoreExpr::fn_(span(0, 1), &["x"], body).unwrap(), "fn"),
            (
                CoreExpr::Apply {
                    span: span(0, 1),
                    func: Box::new(sample_literal()),
                    args: vec![],
                },
                "apply",
            ),
            (
                CoreExpr::If {
                    span: span(0, 1),
                    cond: Box::new(sample_literal()),
                    then_branch: Box::new(sample_literal()),
                    else_branch: Box::new(sample_literal()),
                },
                "if",
            ),
            (CoreExpr::var(span(0, 1), "x").unwrap(), "var"),
            (sample_literal(), "literal"),
            (
                CoreExpr::assign(span(0, 1), "x", sample_literal()).unwrap(),
                "assign",
            ),
            (
                CoreExpr::Let {
                    span: span(0, 1),
                    bindings: vec![],
                    body: Box::new(sample_literal()),
                },
                "let",
            ),
            (
                CoreExpr::perform(span(0, 1), "read", vec![]).unwrap(),
                "perform",
            ),
            (
                CoreExpr::handle(span(0, 1), sample_literal(), vec![]).unwrap(),
                "handle",
            ),
        ];
        for (expr, expected) in &cases {
            assert_eq!(expr.kind_name(), *expected);
            assert_eq!(expr.to_string(), *expected);
        }
        assert_eq!(cases.len(), 9, "all nine primitives must be covered");

        let literal_kinds = [
            (LiteralValue::Int(1), "int"),
            (LiteralValue::Float(1.0), "float"),
            (LiteralValue::Bool(true), "bool"),
            (LiteralValue::String("s".to_owned()), "string"),
            (LiteralValue::Nil, "nil"),
            (
                LiteralValue::Pair(Box::new(LiteralValue::Nil), Box::new(LiteralValue::Nil)),
                "pair",
            ),
        ];
        for (value, expected) in &literal_kinds {
            assert_eq!(value.kind_name(), *expected);
        }
    }

    #[test]
    fn literal_value_display_forms() {
        assert_eq!(LiteralValue::Int(42).to_string(), "42");
        assert_eq!(LiteralValue::Int(-7).to_string(), "-7");
        assert_eq!(LiteralValue::Float(2.5).to_string(), "2.5");
        // Debug-style float formatting keeps the float-ness visible.
        assert_eq!(LiteralValue::Float(3.0).to_string(), "3.0");
        assert_eq!(LiteralValue::Bool(true).to_string(), "true");
        assert_eq!(LiteralValue::Bool(false).to_string(), "false");
        assert_eq!(
            LiteralValue::String("abc".to_owned()).to_string(),
            "\"abc\""
        );
        assert_eq!(LiteralValue::Nil.to_string(), "nil");
        let pair = LiteralValue::Pair(
            Box::new(LiteralValue::Int(1)),
            Box::new(LiteralValue::Int(2)),
        );
        assert_eq!(pair.to_string(), "(1 . 2)");
        let nested = LiteralValue::Pair(Box::new(pair), Box::new(LiteralValue::Int(3)));
        assert_eq!(nested.to_string(), "((1 . 2) . 3)");
    }

    #[test]
    fn span_accessor_returns_constructor_span() {
        let s = span(10, 20);
        let expr = CoreExpr::var(s, "x").unwrap();
        assert_eq!(expr.span(), s);

        let apply = CoreExpr::Apply {
            span: s,
            func: Box::new(CoreExpr::var(s, "f").unwrap()),
            args: vec![sample_literal()],
        };
        assert_eq!(apply.span(), s);
        assert_eq!(apply.span().to_string(), "file#0:10..20");
    }

    #[test]
    fn visit_spans_reports_exact_preorder() {
        // let (100..200) {
        //   x = literal(30..32)   [reused fixture]
        // } in apply(150..200) { var f(150..180), literal(180..200) }
        let program = CoreExpr::Let {
            span: span(100, 200),
            bindings: vec![Binding::new(span(100, 150), "x", sample_literal()).unwrap()],
            body: Box::new(CoreExpr::Apply {
                span: span(150, 200),
                func: Box::new(CoreExpr::var(span(150, 180), "f").unwrap()),
                args: vec![CoreExpr::Literal {
                    span: span(180, 200),
                    value: LiteralValue::Int(7),
                }],
            }),
        };
        let mut collected = Vec::new();
        program.visit_spans(&mut |s| collected.push(s));
        let starts: Vec<u32> = collected.iter().map(|s| s.start.0).collect();
        // Pre-order: let, binding site, binding value, apply, func var, arg.
        assert_eq!(starts, vec![100, 100, 30, 150, 150, 180]);
        assert_eq!(collected.len(), 6);

        // handle(0..50) { body(10..20) } with clauses at 20..30 / 30..50.
        let handled = CoreExpr::handle(
            span(0, 50),
            CoreExpr::var(span(10, 20), "x").unwrap(),
            vec![
                HandlerClause::new(
                    span(20, 30),
                    "read",
                    CoreExpr::var(span(21, 22), "h").unwrap(),
                )
                .unwrap(),
                HandlerClause::new(
                    span(30, 50),
                    "write",
                    CoreExpr::var(span(31, 32), "g").unwrap(),
                )
                .unwrap(),
            ],
        )
        .unwrap();
        let mut spans = Vec::new();
        handled.visit_spans(&mut |s| spans.push(s.start.0));
        // handle, body, clause1 site, clause1 handler, clause2 site,
        // clause2 handler.
        assert_eq!(spans, vec![0, 10, 20, 21, 30, 31]);
    }

    // ---------- negative ----------

    #[test]
    fn fn_constructor_rejects_adjacent_duplicate_params() {
        let err = CoreExpr::fn_(span(0, 1), &["x", "x"], sample_literal())
            .expect_err("adjacent duplicate params must be rejected");
        assert_eq!(
            err,
            CoreError::DuplicateParam {
                name: "x".to_owned()
            }
        );
    }

    #[test]
    fn fn_constructor_rejects_non_adjacent_duplicate_params() {
        let err = CoreExpr::fn_(span(0, 1), &["x", "y", "x"], sample_literal())
            .expect_err("non-adjacent duplicate params must be rejected");
        assert_eq!(
            err,
            CoreError::DuplicateParam {
                name: "x".to_owned()
            }
        );
    }

    #[test]
    fn fn_constructor_rejects_invalid_param_symbol() {
        let err = CoreExpr::fn_(span(0, 1), &["x", "a b"], sample_literal())
            .expect_err("invalid param text must be rejected");
        assert!(matches!(err, CoreError::InvalidSymbol { .. }));
    }

    fn two_read_clauses() -> Vec<HandlerClause> {
        vec![
            HandlerClause::new(span(20, 30), "read", sample_literal()).unwrap(),
            HandlerClause::new(span(30, 40), "read", sample_literal()).unwrap(),
        ]
    }

    #[test]
    fn handle_constructor_rejects_adjacent_duplicate_effects() {
        let err = CoreExpr::handle(span(0, 1), sample_literal(), two_read_clauses())
            .expect_err("adjacent duplicate effect clauses must be rejected");
        assert_eq!(
            err,
            CoreError::DuplicateHandler {
                effect: "read".to_owned()
            }
        );
    }

    #[test]
    fn handle_constructor_rejects_non_adjacent_duplicate_effects() {
        let mut clauses = two_read_clauses();
        clauses.insert(
            1,
            HandlerClause::new(span(40, 50), "write", sample_literal()).unwrap(),
        );
        let err = CoreExpr::handle(span(0, 1), sample_literal(), clauses)
            .expect_err("non-adjacent duplicate effect clauses must be rejected");
        assert_eq!(
            err,
            CoreError::DuplicateHandler {
                effect: "read".to_owned()
            }
        );
    }

    #[test]
    fn handler_clause_rejects_invalid_effect_symbol() {
        let err = HandlerClause::new(span(0, 1), "bad name", sample_literal())
            .expect_err("invalid effect text must be rejected");
        assert!(matches!(err, CoreError::InvalidSymbol { .. }));
    }

    #[test]
    fn var_constructor_rejects_invalid_name() {
        let err = CoreExpr::var(span(0, 1), "a\tb").expect_err("invalid var text must be rejected");
        assert_eq!(
            err,
            CoreError::InvalidSymbol {
                text: "a\tb".to_owned()
            }
        );
    }
}
