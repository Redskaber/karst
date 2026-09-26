//! @path: karst/crates/karst_span/lib.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span
//!
//! Span source-location tracking system and structured diagnostic framework.
//!
//! # Responsibility Boundaries
//!
//! - **What it does:** Provides an immutable `Span` (file_id + byte offset +
//!   expansion generation), a `SourceTable` (file_id -> source text/line
//!   table), and a rustc-style structured `Diagnostic` type.
//! - **What it does not do:** Performs no I/O side effects (rendering returns
//!   a string for the caller to output); depends on no other kerf crate
//!   (Layer 1 infrastructure).
//!
//! # Design Constraints
//!
//! - `Span` is an ID type shared across stages (no prefix).
//! - Errors are data, not exceptions: `Diagnostic` is composable, renderable,
//!   and serializable to text.

pub mod diagnostic;
pub mod source_table;
pub mod span;

use diagnostic::{
    Diagnostic, DiagnosticCode, Severity, SubDiagnostic, Suggestion, render_diagnostic,
};
use source_table::{SourceFile, SourceTable};
use span::{ByteOffset, ExpansionId, FileId, Span};
