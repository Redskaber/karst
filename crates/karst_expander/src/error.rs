//! @path: karst/crates/karst_expander/error.rs
//! @author: redskaber
//! @datetime: 2026-10-04
//! @discription: karst::crates::karst_expander::error

use core::fmt;
use std::error::Error;

use karst_core::CoreError;
use karst_span::Span;
use karst_syntax::Phase;

/// Errors raised during expandtime
#[derive(Debug, Clone, PartialEq)]
pub enum ExpandError {
    // ctx
    ExpansionDepthExceeded { limit: usize, span: Span },
    // expand
    PhaseViolation { phase: Phase, span: Span },
    Core { source: CoreError, span: Span },
    UnboundIdentifier { name: String, span: Span },
    EmptyForm { span: Span },
    MalformedForm { form: &'static str, span: Span },
    TopLevelExpression { span: Span },
}

impl ExpandError {
    pub fn span(&self) -> Span {
        match self {
            ExpandError::ExpansionDepthExceeded { span, .. }
            | ExpandError::PhaseViolation { span, .. }
            | ExpandError::Core { span, .. }
            | ExpandError::UnboundIdentifier { span, .. }
            | ExpandError::EmptyForm { span }
            | ExpandError::MalformedForm { span, .. }
            | ExpandError::TopLevelExpression { span } => *span,
        }
    }
}

impl fmt::Display for ExpandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpandError::ExpansionDepthExceeded { limit, span } => {
                write!(f, "expansion depth exceeded the limit of {limit} at {span}")
            }
            ExpandError::PhaseViolation { phase, span } => {
                write!(
                    f,
                    "phase violation: `{phase}` syntax at {span} is not runtime-phase code"
                )
            }
            ExpandError::Core { source, span } => {
                write!(f, "core construction failed at {span}: {source}")
            }
            ExpandError::UnboundIdentifier { name, span } => {
                write!(
                    f,
                    "unbound identifier `{name}` at {span}: not in any lexical scope and not a Stage 0 global"
                )
            }
            ExpandError::EmptyForm { span } => {
                write!(
                    f,
                    "empty form cannot be expanded: `()` as {span} carries no operator"
                )
            }

            ExpandError::MalformedForm { form, span } => {
                write!(f, "malformed `{form}` form at {span}")
            }
            ExpandError::TopLevelExpression { span } => {
                write!(
                    f,
                    "top-level expression at {span} is not allowed: only `define`, `require` and `module` declarations are valid at module top level"
                )
            }
        }
    }
}

impl Error for ExpandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ExpandError::Core { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the ExpandError rendering surface (sub-stage test
    //! doc FP1; R8 message-shape assertions).

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

    // ---------- negative (error surface) ----------

    #[test]
    fn display_unbound_mentions_name_and_location() {
        let err = ExpandError::UnboundIdentifier {
            name: "fib".to_owned(),
            span: span(7, 10),
        };
        let rendered = err.to_string();
        assert!(rendered.starts_with("unbound identifier `fib`"));
        assert!(rendered.contains("#0:7..10"));
        assert!(!rendered.ends_with('.'));
    }

    #[test]
    fn display_depth_mentions_limit() {
        let err = ExpandError::ExpansionDepthExceeded {
            limit: 512,
            span: span(0, 1),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("512"));
        assert!(rendered.starts_with("expansion depth exceeded"));
        assert!(!rendered.ends_with('.'));
    }

    #[test]
    fn display_phase_violation_mentions_phase() {
        let err = ExpandError::PhaseViolation {
            phase: Phase::ExpandTime,
            span: span(3, 9),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("expandtime"));
        assert!(rendered.contains("runtime-phase"));
        assert!(rendered.starts_with("phase violation"));
    }

    #[test]
    fn display_malformed_mentions_form_name() {
        let err = ExpandError::MalformedForm {
            form: "let",
            span: span(0, 12),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("`let`"));
        assert!(rendered.starts_with("malformed"));
        assert!(!rendered.ends_with('.'));
    }

    #[test]
    fn display_core_wraps_source_and_keeps_span() {
        let source = CoreError::DuplicateParam {
            name: "x".to_owned(),
        };
        let err = ExpandError::Core {
            source,
            span: span(4, 20),
        };
        let rendered = err.to_string();
        assert!(rendered.contains("duplicate parameter `x`"));
        assert!(rendered.contains("#0:4..20"));
        assert_eq!(err.span(), span(4, 20));
        assert!(err.source().is_some());
    }
}
