//! @path: karst/crates/karst_core/module.rs
//! @author: redskaber
//! @datetime: 2026-10-01
//! @discription: karst::crates::karst_core::module
//!
//! the frozen semantic primitive kernel module

use std::fmt;

use karst_span::Span;
use karst_syntax::Symbol;

use crate::{CoreError, CoreExpr, from_syntax_symbol};

/// One top-level declaration
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleItem {
    /// module namespace
    Define {
        span: Span,
        name: Symbol,
        value: CoreExpr,
    },
    /// Dependency declaration
    Require { span: Span, module: Symbol },
    /// nested module
    Module {
        span: Span,
        name: Symbol,
        items: Vec<ModuleItem>,
    },
}

impl ModuleItem {
    pub fn span(&self) -> Span {
        match self {
            ModuleItem::Define { span, .. }
            | ModuleItem::Require { span, .. }
            | ModuleItem::Module { span, .. } => *span,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            ModuleItem::Define { .. } => "define",
            ModuleItem::Require { .. } => "require",
            ModuleItem::Module { .. } => "module",
        }
    }

    /// Pre-order span traversal: this item's span first, then nested
    pub fn visit_spans(&self, f: &mut impl FnMut(Span)) {
        match self {
            ModuleItem::Define { span, value, .. } => {
                f(*span);
                value.visit_spans(f);
            }
            ModuleItem::Require { span, .. } => f(*span),
            ModuleItem::Module { span, items, .. } => {
                f(*span);
                for item in items {
                    item.visit_spans(f);
                }
            }
        }
    }

    pub fn define(span: Span, name: &str, value: CoreExpr) -> Result<ModuleItem, CoreError> {
        Ok(ModuleItem::Define {
            span,
            name: from_syntax_symbol(Symbol::new(name))?,
            value,
        })
    }

    pub fn require(span: Span, module: &str) -> Result<ModuleItem, CoreError> {
        Ok(ModuleItem::Require {
            span,
            module: from_syntax_symbol(Symbol::new(module))?,
        })
    }

    pub fn module(span: Span, name: &str, items: Vec<ModuleItem>) -> Result<ModuleItem, CoreError> {
        Ok(ModuleItem::Module {
            span,
            name: from_syntax_symbol(Symbol::new(name))?,
            items,
        })
    }
}

impl fmt::Display for ModuleItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.kind_name())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the ModuleItem declaration layer (sub-stage test doc FP2/FP4)

    use super::*;
    use crate::expr::LiteralValue;
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

    fn literal(start: u32, end: u32) -> CoreExpr {
        CoreExpr::Literal {
            span: span(start, end),
            value: LiteralValue::Int(1),
        }
    }

    // ---------- positive ----------

    #[test]
    fn moduleitem_kinds_spans_and_nested_visit() {
        let program = vec![
            ModuleItem::require(span(0, 8), "math").unwrap(),
            ModuleItem::module(
                span(10, 60),
                "inner",
                vec![
                    ModuleItem::define(span(20, 40), "one", literal(30, 32)).unwrap(),
                    ModuleItem::define(span(40, 60), "two", literal(50, 52)).unwrap(),
                ],
            )
            .unwrap(),
        ];
        let kinds: Vec<&'static str> = program.iter().map(|i| i.kind_name()).collect();
        assert_eq!(kinds, vec!["require", "module"]);
        assert_eq!(program[0].span(), span(0, 8));
        assert_eq!(program[0].to_string(), "require");

        let mut collected = Vec::new();
        for item in &program {
            item.visit_spans(&mut |s| collected.push(s.start.0));
        }
        // require; module; inner define1, its literal; inner define2, its
        // literal.
        assert_eq!(collected, vec![0, 10, 20, 30, 40, 50]);
    }

    // ---------- negative ----------

    #[test]
    fn define_constructor_rejects_invalid_name() {
        let err = ModuleItem::define(span(0, 1), "bad name", literal(0, 1))
            .expect_err("invalid define name must be rejected");
        assert!(matches!(err, CoreError::InvalidSymbol { .. }));
    }

    #[test]
    fn require_constructor_rejects_invalid_module_name() {
        let err =
            ModuleItem::require(span(0, 1), "").expect_err("empty module name must be rejected");
        assert_eq!(
            err,
            CoreError::InvalidSymbol {
                text: String::new()
            }
        );
    }

    #[test]
    fn module_constructor_rejects_invalid_name() {
        let err = ModuleItem::module(span(0, 1), "a b", vec![])
            .expect_err("invalid module name must be rejected");
        assert!(matches!(err, CoreError::InvalidSymbol { .. }));
    }
}
