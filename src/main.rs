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

    println!("{}", text);
}
