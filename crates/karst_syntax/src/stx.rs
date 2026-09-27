//! @path: karst/crates/karst_syntax/stx.rs
//! @author: redskaber
//! @datetime: 2026-09-27
//! @discription: karst::crates::karst_syntax::stx

use std::rc::Rc;

use crate::{Phase, scope::ScopeSet, symbol::Symbol};
use karst_span::span::Span;

/// syntax literal
#[derive(Debug, Clone, PartialEq)]
pub enum StxLiteral {
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
}

impl StxLiteral {
    pub fn render(&self) -> String {
        match self {
            StxLiteral::Int(i) => i.to_string(),
            StxLiteral::Float(f) => {
                if f.fract() == 0.0 && f.is_finite() {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            StxLiteral::Str(s) => format!("{:?}", s),
            StxLiteral::Bool(b) => b.to_string(),
            StxLiteral::Nil => "nil".to_string(),
        }
    }
}

/// syntax datum
#[derive(Debug, Clone, PartialEq)]
pub enum StxDatum {
    /// syntax symbol
    Symbol(Symbol),
    /// syntax literal
    Literal(StxLiteral),
    /// syntax list
    List(Vec<Stx>),
    /// syntax vector
    Vector(Vec<Stx>),
}

impl StxDatum {
    pub fn as_symbol(&self) -> Option<Symbol> {
        match self {
            StxDatum::Symbol(s) => Some(*s),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[Stx]> {
        match self {
            StxDatum::List(items) => Some(items),
            _ => None,
        }
    }

    pub fn list_head_symbol(&self) -> Option<Symbol> {
        self.as_list()
            .and_then(|items| items.first())
            .and_then(|stx| stx.datum.as_symbol())
    }
}

/// syntax object
#[derive(Debug, Clone, PartialEq)]
pub struct Stx {
    /// syntax expr 'data'
    pub datum: StxDatum,
    /// syntax span 'position'
    pub span: Span,
    /// syntax scopes
    pub scopes: ScopeSet,
    /// syntax phase
    pub phase: Phase,
}

impl Stx {
    pub fn new(datum: StxDatum, span: Span, scopes: ScopeSet, phase: Phase) -> Self {
        Stx {
            datum,
            span,
            scopes,
            phase,
        }
    }

    pub fn symbol(sym: Symbol, span: Span, scopes: ScopeSet) -> Self {
        Stx::new(StxDatum::Symbol(sym), span, scopes, Phase::Runtime)
    }

    pub fn list(items: Vec<Stx>, span: Span, scopes: ScopeSet) -> Self {
        Stx::new(StxDatum::List(items), span, scopes, Phase::Runtime)
    }

    pub fn literal(value: StxLiteral, span: Span, scopes: ScopeSet) -> Self {
        Stx::new(StxDatum::Literal(value), span, scopes, Phase::Runtime)
    }

    pub fn as_macro_expansion(&self, macro_def_scopes: &ScopeSet) -> Self {
        Stx::new(
            self.datum.clone(),
            self.span.bumped_expansion(),
            self.scopes.union(macro_def_scopes),
            Phase::ExpandTime,
        )
    }

    /// render syntax object (dispatch)
    pub fn render(&self, resolve: &dyn Fn(Symbol) -> String) -> String {
        match &self.datum {
            // datum => expr
            StxDatum::Symbol(s) => resolve(*s),
            StxDatum::Literal(l) => l.render(),
            StxDatum::List(items) => {
                let inner: Vec<String> = items.iter().map(|s| s.render(resolve)).collect();
                format!("({})", inner.join(" ")) // (1 2 3 4)
            }
            StxDatum::Vector(items) => {
                let inner: Vec<String> = items.iter().map(|s| s.render(resolve)).collect();
                format!("[{}]", inner.join(" ")) // [1 2 3 4]
            }
        }
    }

    pub fn total_span(&self) -> Span {
        match &self.datum {
            StxDatum::List(items) | StxDatum::Vector(items) => {
                let mut span = self.span;
                for item in items {
                    span = span.merge(item.span);
                }
                span
            }
            _ => self.span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scope::ScopeId;

    fn scopes(ids: &[ScopeId]) -> ScopeSet {
        ScopeSet::from_iter_scopes(ids.iter().copied())
    }

    #[test]
    fn render_list_nested() {
        let s = Stx::list(
            // (1(symbol) 2(int))
            vec![
                Stx::symbol(Symbol(1), Span::dummy(), scopes(&[])),
                Stx::literal(StxLiteral::Int(2), Span::dummy(), scopes(&[])),
            ],
            Span::dummy(),
            scopes(&[]),
        );
        assert_eq!(s.render(&|sym: Symbol| format!("#{}", sym.0)), "(#1 2)");
    }

    #[test]
    fn macro_expansion_bumps_phase_and_scopes() {
        let base = Stx::symbol(Symbol(3), Span::new(0, 1, 5), scopes(&[1]));
        let def_scopes = scopes(&[9]);
        let expanded = base.as_macro_expansion(&def_scopes);
        assert_eq!(expanded.phase, Phase::ExpandTime);
        assert_eq!(expanded.span.expansion_id, 1);
        assert!(expanded.scopes.contains(1) && expanded.scopes.contains(9)); // union
    }

    #[test]
    fn list_head_symbol_extraction() {
        let s = Stx::list(
            vec![
                Stx::symbol(Symbol(7), Span::dummy(), scopes(&[])),
                Stx::symbol(Symbol(8), Span::dummy(), scopes(&[])),
            ],
            Span::dummy(),
            scopes(&[]),
        );
        assert_eq!(s.datum.list_head_symbol(), Some(Symbol(7)));
        assert_eq!(
            Stx::symbol(Symbol(7), Span::dummy(), scopes(&[]))
                .datum
                .list_head_symbol(), // handle only list
            None
        );
    }

    #[test]
    fn literal_render() {
        assert_eq!(StxLiteral::Int(-5).render(), "-5");
        assert_eq!(StxLiteral::Bool(true).render(), "true");
        assert_eq!(StxLiteral::Nil.render(), "nil");
        assert_eq!(StxLiteral::Float(1.5).render(), "1.5");
    }
}
