//! @path: karst/crates/karst_span/lib.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span

pub mod source_map;
pub mod span;

use source_map::{SourceFile, SourceTable};
use span::{ByteOffset, ExpansionId, FileId, Span};
