//! @path: karst/crates/karst_core/expr.rs
//! @author: redskaber
//! @datetime: 2026-10-01
//! @discription: karst::crates::karst_core::expr
//!
//! CoreExpr: the frozen semantic primitive expression ADT

use core::fmt;
use std::collections::HashSet;

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

/// Let Binding
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

/// Handle Clause
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
            CoreExpr::If { .. } => "fn",
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

    /// fn constructor
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
    // more ..
}
