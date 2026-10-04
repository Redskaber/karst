//! @path: karst/crates/karst_syntax/stx.rs
//! @author: redskaber
//! @datetime: 2026-09-27
//! @discription: karst::crates::karst_syntax::stx
//!
//! syntax object `Stx` (Syntax Object): Reader input, Expander input
//! simple design and impl

use std::fmt;

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
            StxDatum::String(_) => "string",
            StxDatum::List(_) => "list",
        }
    }
}

impl fmt::Display for StxDatum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StxDatum::Symbol(s) => f.write_str(s.as_str()),
            StxDatum::Int(v) => write!(f, "{v}"),
            StxDatum::Float(v) => write!(f, "{v:?}"),
            StxDatum::Bool(v) => write!(f, "{v}"),
            StxDatum::String(v) => write!(f, "{v:?}"),
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
    //! Unit tests for the StxDatum/SyntaxObject feature point (sub-stage
    //! test doc FP4).

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

    fn empty() -> (Span, ScopeSet, Phase) {
        (span(0, 1), ScopeSet::new(), Phase::Runtime)
    }

    // ---------- positive ----------

    #[test]
    fn datum_kind_names_and_display_forms() {
        let (s, scopes, phase) = empty();
        let cases: [(SyntaxObject, &str, &str); 6] = [
            (
                SyntaxObject::symbol("add", s, scopes.clone(), phase).unwrap(),
                "symbol",
                "add",
            ),
            (SyntaxObject::int(42, s, scopes.clone(), phase), "int", "42"),
            (
                SyntaxObject::float(2.5, s, scopes.clone(), phase),
                "float",
                "2.5",
            ),
            (
                SyntaxObject::bool(true, s, scopes.clone(), phase),
                "bool",
                "true",
            ),
            (
                SyntaxObject::string("s".to_owned(), s, scopes.clone(), phase),
                "string",
                "\"s\"",
            ),
            (SyntaxObject::list(vec![], s, scopes, phase), "list", "()"),
        ];
        for (object, kind, rendered) in cases {
            assert_eq!(object.datum.kind_name(), kind);
            assert_eq!(object.datum.to_string(), rendered);
        }
        // Nested rendering through child objects.
        let (s, scopes, phase) = empty();
        let list = SyntaxObject::list(
            vec![
                SyntaxObject::symbol("add", s, ScopeSet::new(), Phase::Runtime).unwrap(),
                SyntaxObject::int(1, s, ScopeSet::new(), Phase::Runtime),
                SyntaxObject::int(2, s, ScopeSet::new(), Phase::Runtime),
            ],
            s,
            scopes,
            phase,
        );
        assert_eq!(list.datum.to_string(), "(add 1 2)");
    }

    #[test]
    fn span_preorder_and_deep_scope_flip() {
        // Tree (f x 42): the visit order is the list node first, then
        // children left-to-right (the 0.4 expander consumption
        // contract, mirroring CoreExpr::visit_spans).
        let mk = |start: u32, end: u32| span(start, end);
        let head = SyntaxObject::symbol("f", mk(1, 2), ScopeSet::new(), Phase::Runtime).unwrap();
        let arg_name =
            SyntaxObject::symbol("x", mk(3, 4), ScopeSet::new(), Phase::Runtime).unwrap();
        let arg_lit = SyntaxObject::int(42, mk(5, 7), ScopeSet::new(), Phase::Runtime);
        let list = SyntaxObject::list(
            vec![head, arg_name, arg_lit],
            mk(0, 8),
            ScopeSet::new(),
            Phase::Runtime,
        );

        let mut order = Vec::new();
        list.visit_spans(&mut |s| order.push((s.start.0, s.end.0)));
        assert_eq!(order, vec![(0, 8), (1, 2), (3, 4), (5, 7)]);

        // Deep functional flip: the receiver stays untouched, every
        // node of the copy gains the scope, spans and datum survive.
        let flipped = list.with_scope_added(ScopeId(7));
        let mut flipped_spans = Vec::new();
        flipped.visit_spans(&mut |s| flipped_spans.push((s.start.0, s.end.0)));
        assert_eq!(flipped_spans, order, "spans unchanged by the flip");
        assert_eq!(flipped.datum.to_string(), "(f x 42)");
        let StxDatum::List(items) = &flipped.datum else {
            panic!("datum shape must survive the flip");
        };
        for item in items {
            assert!(item.scopes.contains(ScopeId(7)));
        }
        // Original tree: no node gained the scope (principle 23).
        let StxDatum::List(original) = &list.datum else {
            panic!("receiver datum must be intact");
        };
        assert!(!list.scopes.contains(ScopeId(7)));
        for item in original {
            assert!(!item.scopes.contains(ScopeId(7)));
        }
    }

    // ---------- negative ----------

    #[test]
    fn symbol_constructor_rejects_empty_text() {
        let (s, scopes, phase) = empty();
        let err = SyntaxObject::symbol("", s, scopes, phase)
            .expect_err("empty identifier must be rejected via the object path");
        assert_eq!(
            err,
            SyntaxError::InvalidSymbol {
                text: String::new()
            }
        );
    }

    #[test]
    fn symbol_constructor_rejects_interior_space() {
        let (s, scopes, phase) = empty();
        assert!(matches!(
            SyntaxObject::symbol("a b", s, scopes, phase),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn symbol_constructor_rejects_leading_space() {
        let (s, scopes, phase) = empty();
        assert!(matches!(
            SyntaxObject::symbol(" x", s, scopes, phase),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn symbol_constructor_rejects_control_char() {
        let (s, scopes, phase) = empty();
        assert!(matches!(
            SyntaxObject::symbol("a\u{0}b", s, scopes, phase),
            Err(SyntaxError::InvalidSymbol { .. })
        ));
    }

    #[test]
    fn reversed_span_rejected_before_object_construction() {
        // Structural no-DUMMY guard: a malformed span fails at the
        // karst-span constructor and never reaches a syntax object.
        let err = Span::root(FileId(0), ByteOffset(10), ByteOffset(2))
            .expect_err("reversed span must be rejected before object construction");
        assert!(matches!(err, karst_span::SpanError::StartAfterEnd { .. }));
    }

    #[test]
    fn scope_flip_never_duplicates() {
        // Set-semantics guard: flipping the same scope twice must not
        // duplicate it anywhere in the tree.
        let (s, scopes, phase) = empty();
        let leaf = SyntaxObject::symbol("x", s, scopes, phase).unwrap();
        let list = SyntaxObject::list(vec![leaf], s, ScopeSet::new(), Phase::Runtime);
        let once = list.with_scope_added(ScopeId(1));
        let twice = once.with_scope_added(ScopeId(1));
        assert_eq!(twice.scopes.len(), 1);
        let StxDatum::List(items) = &twice.datum else {
            panic!("list datum expected");
        };
        assert_eq!(items[0].scopes.len(), 1);
    }
}
