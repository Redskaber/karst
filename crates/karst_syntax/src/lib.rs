//! @path: karst/crates/karst_syntax/lib.rs
//! @author: redskaber
//! @datetime: 2026-09-26
//! @discription: karst::crates::karst_syntax

pub mod error;
pub mod expr;
pub mod phase;
pub mod scope;
pub mod symbol; // plan move to karst_span

pub use error::SyntaxError;
pub use expr::{StxDatum, SyntaxObject};
pub use phase::Phase;
pub use scope::{ScopeId, ScopeSet};
pub use symbol::Symbol;
