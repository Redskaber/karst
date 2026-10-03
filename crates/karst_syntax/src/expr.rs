//! @path: karst/crates/karst_syntax/stx.rs
//! @author: redskaber
//! @datetime: 2026-09-27
//! @discription: karst::crates::karst_syntax::stx
//!
//! syntax object `Stx` (Syntax Object): Reader input, Expander input
//! simple design and impl

use core::fmt;

use crate::{Phase, ScopeId, SyntaxError, scope::ScopeSet, symbol::Symbol};
use karst_span::span::Span;

/// syntax datum
#[derive(Debug, Clone, PartialEq)]
pub enum StxDatum {
    /// syntax symbol
    Symbol(Symbol),
    /// Integer Literal
    Int(i64),
    /// Floating-point Literal
    Float(f64),
    /// Boolean literal
    Bool(bool),
    /// String Literal
    String(String),
    /// syntax list
    List(Vec<SyntaxObject>),
}

impl StxDatum {
    /// Stable diagnostic name
    pub fn kind_name(&self) -> &'static str {
        match self {
            StxDatum::Symbol(_) => "symbol",
            StxDatum::Int(_) => "int",
            StxDatum::Float(_) => "float",
            StxDatum::Bool(_) => "bool",
            StxDatum::String(_) => "String",
            StxDatum::List(_) => "list",
        }
    }
}

impl fmt::Display for StxDatum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StxDatum::Symbol(s) => f.write_str(s.as_str()),
            StxDatum::Int(v) => write!(f, "{v}"),
            StxDatum::Float(v) => write!(f, "{v}"),
            StxDatum::Bool(v) => write!(f, "{v}"),
            StxDatum::String(v) => write!(f, "{v}"),
            StxDatum::List(items) => {
                f.write_str("(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{}", item.datum)?;
                }
                f.write_str(")")
            }
        }
    }
}

/// syntax object
#[derive(Debug, Clone, PartialEq)]
pub struct SyntaxObject {
    /// syntax expr 'data'
    pub datum: StxDatum,
    /// syntax span 'position'
    pub span: Span,
    /// syntax scopes
    pub scopes: ScopeSet,
    /// syntax phase
    pub phase: Phase,
}

impl SyntaxObject {
    pub fn new(datum: StxDatum, span: Span, scopes: ScopeSet, phase: Phase) -> Self {
        SyntaxObject {
            datum,
            span,
            scopes,
            phase,
        }
    }

    pub fn symbol(
        text: &str,
        span: Span,
        scopes: ScopeSet,
        phase: Phase,
    ) -> Result<SyntaxObject, SyntaxError> {
        Ok(SyntaxObject::new(
            StxDatum::Symbol(Symbol::new(text)?),
            span,
            scopes,
            phase,
        ))
    }
    pub fn int(value: i64, span: Span, scopes: ScopeSet, phase: Phase) -> SyntaxObject {
        SyntaxObject::new(StxDatum::Int(value), span, scopes, phase)
    }

    pub fn float(value: f64, span: Span, scopes: ScopeSet, phase: Phase) -> SyntaxObject {
        SyntaxObject::new(StxDatum::Float(value), span, scopes, phase)
    }

    pub fn bool(value: bool, span: Span, scopes: ScopeSet, phase: Phase) -> SyntaxObject {
        SyntaxObject::new(StxDatum::Bool(value), span, scopes, phase)
    }
    pub fn string(value: String, span: Span, scopes: ScopeSet, phase: Phase) -> SyntaxObject {
        SyntaxObject::new(StxDatum::String(value), span, scopes, phase)
    }

    pub fn list(
        items: Vec<SyntaxObject>,
        span: Span,
        scopes: ScopeSet,
        phase: Phase,
    ) -> SyntaxObject {
        SyntaxObject::new(StxDatum::List(items), span, scopes, phase)
    }

    /// Pre-order span traversal
    pub fn visit_spans(&self, f: &mut impl FnMut(Span)) {
        f(self.span);
        if let StxDatum::List(items) = &self.datum {
            for item in items {
                item.visit_spans(f);
            }
        }
    }

    /// Deep functional scope
    pub fn with_scope_added(&self, scope_id: ScopeId) -> SyntaxObject {
        let mut out = self.clone();
        out.add_scope_deep(scope_id);
        out
    }

    /// Private deep mutation hepler
    fn add_scope_deep(&mut self, scope_id: ScopeId) {
        self.scopes.insert(scope_id);
        if let StxDatum::List(items) = &mut self.datum {
            for item in items {
                item.add_scope_deep(scope_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use karst_span::{ByteOffset, ExpansionId, FileId};

    use super::*;

    #[test]
    fn symbol_constructor_reject_empty_text() {
        let span = Span::new(FileId(0), ByteOffset(0), ByteOffset(1), ExpansionId::ROOT)
            .expect("test fiture span");
        let scopes = ScopeSet::new();
        let phase = Phase::Runtime;
        assert!(matches!(
            SyntaxObject::symbol("a\u{0}b", span, scopes, phase),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    // more ...
}
