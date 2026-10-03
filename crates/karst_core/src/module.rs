//! @path: karst/crates/karst_core/module.rs
//! @author: redskaber
//! @datetime: 2026-10-01
//! @discription: karst::crates::karst_core::module
//!
//! the frozen semantic primitive kernel module

use core::fmt;

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
    // more ...
}
