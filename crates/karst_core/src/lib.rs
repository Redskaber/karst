//! @path: karst/crates/karst_core/lib.rs
//! @author: redskaber
//! @datetime: 2026-10-01
//! @discription: karst::crates::karst_core
//!
//! the frozen semantic primitive kernel

pub mod error;
pub mod expr;
pub mod module;

pub use error::{CoreError, from_syntax_symbol};
pub use expr::{Binding, CoreExpr, HandlerClause, LiteralValue};
pub use module::ModuleItem;
