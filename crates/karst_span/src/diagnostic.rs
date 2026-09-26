//! @path: karst/crates/karst_span/diagnostic.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span::diagnostic
//! Structured diagnostic framework.
//!
//! Inspired by rustc's `Diagnostic` structure:
//! - **Errors are data, not exceptions**;
//! - Supports error-recovery strategies (diagnostics can be collected and rendered together);
//! - Multi-stage error association (`children` sub-diagnostics + `suggestions` fixes).
//!
//! The minimal shared representation for all error types is `{ message: String, span: Span }`.

use core::fmt;

use crate::{source_table::SourceTable, span::Span};

/// diagnostic level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// diagnostic id
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(pub u32);

impl DiagnosticCode {
    /// render format
    /// ```text
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
    pub suggestions: Vec<Suggestion>,
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
            suggestions: Vec::new(),
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
    /// ```text
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
    /// ```text
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
        self.suggestions.push(Suggestion {
            message: message.into(),
            span,
            replacement: replacement.into(),
        });
        self
    }

    pub fn is_error(&self) -> bool {
        matches!(self.severity, Severity::Error)
    }
}

/// render diagnostic
/// format:
/// ```text
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
    for sugg in &diag.suggestions {
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

    const SAMPLE_FILE: &str = "t.krt";
    const SAMPLE_SRC: &str = "(+ # 1)";

    /// Creates a fresh `SourceTable` containing the shared sample source.
    macro_rules! sample_source_table {
        () => {{
            let mut source_table = SourceTable::new();
            let file_id = source_table.add_file(SAMPLE_FILE, SAMPLE_SRC);
            (source_table, file_id)
        }};
    }

    /// Builds a diagnostic that exercises all diagnostic components:
    /// - primary diagnostic (`Error` with code)
    /// - child diagnostic (`Note`)
    /// - suggestion (`Help`)
    fn build_full_sample(primary: Span, child: Span, suggestion: Span) -> Diagnostic {
        Diagnostic::error(Some(DiagnosticCode(1)), "invalid character '#'", primary)
            .with_child(Severity::Note, "expected an operand here", child)
            .with_suggestion("did you mean to enter `1`?", suggestion, "1")
    }

    fn assert_contains(haystack: &str, needle: &str) {
        assert!(
            haystack.contains(needle),
            "expected output to contain {:?}, but got:\n{}",
            needle,
            haystack
        );
    }

    fn assert_not_contains(haystack: &str, needle: &str) {
        assert!(
            !haystack.contains(needle),
            "expected output not to contain {:?}, but got:\n{}",
            needle,
            haystack
        );
    }

    /// Full rendering: primary diagnostic, child diagnostic, suggestion, and source excerpt.
    #[test]
    fn diagnostic_render_full_shape() {
        let (source_table, file_id) = sample_source_table!();
        let diagnostic = build_full_sample(
            Span::new(file_id, 3, 4),
            Span::new(file_id, 0, 7),
            Span::new(file_id, 3, 4),
        );

        let output = render_diagnostic(&diagnostic, &source_table);

        assert_contains(&output, "error[E0001]: invalid character '#'");
        assert_contains(&output, "--> t.krt:1:4");
        assert_contains(&output, SAMPLE_SRC);
        assert_contains(&output, "note: expected an operand here");
        assert_contains(&output, "help: did you mean to enter `1`?");
    }

    /// A diagnostic without a code must not render `[EXXXX]`.
    #[test]
    fn diagnostic_render_without_code_omits_brackets() {
        let (source_table, file_id) = sample_source_table!();
        let diagnostic = Diagnostic::error(None, "invalid character '#'", Span::new(file_id, 3, 4));

        let output = render_diagnostic(&diagnostic, &source_table);

        assert!(output.starts_with("error: invalid character '#'"));
        assert_not_contains(&output, "error[");
    }

    /// Warning-level diagnostics use the same rendering path.
    #[test]
    fn diagnostic_render_warning_severity_label() {
        let (source_table, file_id) = sample_source_table!();
        let diagnostic = Diagnostic::warning(
            Some(DiagnosticCode(331)),
            "unused binding",
            Span::new(file_id, 0, 1),
        );

        let output = render_diagnostic(&diagnostic, &source_table);

        assert_contains(&output, "warning[E0331]: unused binding");
    }

    /// No children and no suggestions must not leak `note:` or `help:` lines.
    #[test]
    fn diagnostic_render_no_children_no_suggestions() {
        let (source_table, file_id) = sample_source_table!();
        let diagnostic = Diagnostic::error(
            Some(DiagnosticCode(1)),
            "invalid character '#'",
            Span::new(file_id, 3, 4),
        );

        let output = render_diagnostic(&diagnostic, &source_table);

        assert_not_contains(&output, "note:");
        assert_not_contains(&output, "help:");
    }

    /// `is_error` returns true only for the `Error` severity.
    #[test]
    fn diagnostic_is_error_matches_severity() {
        let (_, file_id) = sample_source_table!();

        let error = Diagnostic::error(None, "error", Span::new(file_id, 0, 1));
        let warning = Diagnostic::warning(None, "warning", Span::new(file_id, 0, 1));

        assert!(error.is_error());
        assert!(!warning.is_error());
    }

    /// `Display` and `label` must agree on the severity text.
    #[test]
    fn severity_label_and_display_match() {
        for (severity, expected) in [
            (Severity::Error, "error"),
            (Severity::Warning, "warning"),
            (Severity::Note, "note"),
            (Severity::Help, "help"),
        ] {
            assert_eq!(severity.label(), expected);
            assert_eq!(format!("{}", severity), expected);
        }
    }

    /// Diagnostic codes are zero-padded to four digits and are not truncated beyond four digits.
    #[test]
    fn diagnostic_code_render_zero_pads_to_four_digits() {
        assert_eq!(DiagnosticCode(1).render(), "E0001");
        assert_eq!(DiagnosticCode(331).render(), "E0331");
        assert_eq!(DiagnosticCode(9999).render(), "E9999");
        assert_eq!(DiagnosticCode(10_000).render(), "E10000");
    }
}
