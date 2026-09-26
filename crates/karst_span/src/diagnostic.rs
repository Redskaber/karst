//! @path: karst/crates/karst_span/diagnostic.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span::diagnostic

use core::fmt;

use crate::{
    source_table::{self, SourceTable},
    span::{self, Span},
};

/// diagnostic level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
            Severity::Help => write!(f, "help"),
        }
    }
}

/// diagnostic id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(pub u32);

impl DiagnosticCode {
    /// render format
    /// ```
    /// E0001
    /// ```
    pub fn render(self) -> String {
        format!("E{:04}", self.0)
    }
}

/// Sub Diagnostic
#[derive(Debug, Clone)]
pub struct SubDiagnostic {
    /// diagnostic level
    pub severity: Severity,
    /// diagnostic message
    pub message: String,
    /// diagnostic position
    pub span: Span,
}

/// fix tips
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// suggestion message
    pub message: String,
    /// suggestion position
    pub span: Span,
    /// suggestion replacement (fix tips value)
    pub replacement: String,
}

/// diagnostic full structure
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// diagnostic level
    pub severity: Severity,
    /// diagnostic code [error0001]
    pub code: Option<DiagnosticCode>,
    /// diagnostic message
    pub message: String,
    /// diagnostic position
    pub span: Span,
    /// child diagnostic
    pub children: Vec<SubDiagnostic>,
    /// diagnostic fix tips
    pub suggestion: Vec<Suggestion>,
}

impl Diagnostic {
    pub fn new(
        severity: Severity,
        code: Option<DiagnosticCode>,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        Diagnostic {
            severity,
            code,
            message: message.into(),
            span,
            children: Vec::new(),
            suggestion: Vec::new(),
        }
    }

    /// diagnostic error quick entry
    pub fn error(code: Option<DiagnosticCode>, message: impl Into<String>, span: Span) -> Self {
        Diagnostic::new(Severity::Error, code, message, span)
    }

    /// diagnostic warning quick entry
    pub fn warning(code: Option<DiagnosticCode>, message: impl Into<String>, span: Span) -> Self {
        Diagnostic::new(Severity::Warning, code, message, span)
    }

    /// extra sub diagnostic (child-link)
    /// ```
    /// diagnostic
    ///  | (sub-diagnostic field)
    ///  []-[]-[]...
    /// ````
    pub fn with_child(
        mut self,
        severity: Severity,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        self.children.push(SubDiagnostic {
            severity,
            message: message.into(),
            span,
        });
        self
    }

    /// extra suggestion (child-link)
    /// ```
    /// diagnostic
    ///  | (suggestion field)
    ///  []-[]-[]...
    /// ````
    pub fn with_suggestion(
        mut self,
        message: impl Into<String>,
        span: Span,
        replacement: impl Into<String>,
    ) -> Self {
        self.suggestion.push(Suggestion {
            message: message.into(),
            span,
            replacement: replacement.into(),
        });
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// render diagnostic
/// format:
/// ```
/// error[E0001]: invaild char '#'
///   --> main.krf:1:5
///    |
///  1 | (+ # 1)
///    |    ^
///  suberror part
///  help part
/// ```
pub fn render_diagnostic(diag: &Diagnostic, source_table: &SourceTable) -> String {
    let mut out = String::new();

    // diagnostic code: E0001 => [E0001]
    let code = diag
        .code
        .as_ref()
        .map(|c| format!("[{}]", c.render()))
        .unwrap_or_default();
    out.push_str(&format!("{}{}: {}\n", diag.severity, code, diag.message)); // out:
    // error[E0001]: invaild char '#'

    out.push_str(&format!(
        "  --> {}\n",
        source_table.render_location(diag.span.file_id, diag.span.start, diag.span.expansion_id)
    )); // out:  --> main.krt:3:5

    if !diag.span.is_empty() || source_table.file(diag.span.file_id).is_some() {
        // out:
        // ```
        //    0 | (+ # 1)
        //      |    ^
        // ```
        let excerpt = source_table.excerpt(diag.span.file_id, diag.span.start);
        if !excerpt.is_empty() {
            out.push_str("     |\n");
            for line in excerpt.lines() {
                out.push_str(&format!("{}\n", line));
            }
        }
    }

    // error: invaild char '#' (main.krt:3:5)
    for child in &diag.children {
        out.push_str(&format!(
            "  {}: {} ({})\n",
            child.severity,
            child.message,
            source_table.render_location(
                child.span.file_id,
                child.span.start,
                child.span.expansion_id
            )
        ));
    }

    // help: replace char '#' ->  「1 」
    for sugg in &diag.suggestion {
        out.push_str(&format!(
            "  help: {} -> 「{} 」\n",
            sugg.message, sugg.replacement
        ))
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_render_full_shape() {
        let mut sm = SourceTable::new();
        let id = sm.add_file("t.krt", "(+ # 1)");
        let diag = Diagnostic::error(
            Some(DiagnosticCode(1)),
            "invaild char '#'",
            Span::new(id, 3, 4),
        )
        .with_child(Severity::Note, "this need oprand", Span::new(id, 0, 7))
        .with_suggestion("do you need entry 1", Span::new(id, 3, 4), "1");
        let text = render_diagnostic(&diag, &sm);
        assert!(text.contains("error[E0001]: invaild char '#'"));
        assert!(text.contains("--> t.krt:1:4"));
        assert!(text.contains("note: this need oprand"));
        assert!(text.contains("help: do you need entry 1"));
    }

    #[test]
    fn code_render() {
        assert_eq!(DiagnosticCode(1).render(), "E0001");
        assert_eq!(DiagnosticCode(331).render(), "E0331");
    }
}
