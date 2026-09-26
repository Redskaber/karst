//! @path: karst/crates/karst_span/lib.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span

pub mod diagnostic;
pub mod source_table;
pub mod span;

use diagnostic::{
    Diagnostic, DiagnosticCode, Severity, SubDiagnostic, Suggestion, render_diagnostic,
};
use source_table::{SourceFile, SourceTable};
use span::{ByteOffset, ExpansionId, FileId, Span};
