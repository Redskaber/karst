//! @path: karst/crates/karst_span/lib.rs
//! @author: redskaber
//! @datetime: 2026-09-25
//! @discription: karst::crates::karst_span
//!
//! source position tracking for the karst compiler pipline

pub mod span;

pub use span::{ByteOffset, ExpansionId, FileId, Span, SpanError};
