//! @path: karst/src/lib.rs
//! @author: redskaber
//! @datetime: 2026-10-03
//! @discription: karst::lib
//!
//! karst root aggregation crate of th karst compiler workspace

pub use karst_reader as reader;
pub use karst_span as span;
pub use karst_syntax as syntax;

/// workspace version of the karst compiler
pub const WORKSPACE_VERSION: &str = env!("CARGO_PKG_VERSION");
