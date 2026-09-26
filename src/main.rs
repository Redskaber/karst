//! @path: karst/src/main.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::main

use karst_span::{
    diagnostic::{Diagnostic, DiagnosticCode, Severity, render_diagnostic},
    source_table::SourceTable,
    span::Span,
};

fn main() {
    let mut source_table = SourceTable::new();
    let file_id = source_table.add_file("main.krt", "(+ # 1)");

    let diagnostic = Diagnostic::error(
        Some(DiagnosticCode(1)),
        "invalid character '#'",
        Span::new(file_id, 3, 4),
    )
    .with_child(
        Severity::Note,
        "expected an operand here",
        Span::new(file_id, 0, 7),
    )
    .with_suggestion("did you mean to enter `1`?", Span::new(file_id, 3, 4), "1");

    let rendered = render_diagnostic(&diagnostic, &source_table);
    print!("{}", rendered);

    if diagnostic.is_error() {
        std::process::exit(1);
    }
}
