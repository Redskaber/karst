//! @path: karst/crates/karst_syntax/lib.rs
//! @author: redskaber
//! @datetime: 2026-09-26
//! @discription: karst::crates::karst_syntax

mod scope;
mod stx;
mod symbol;

use scope::{ScopeId, ScopeSet};
use stx::{Stx, StxDatum, StxLiteral};
use symbol::{Keyword, Symbol, SymbolTable};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// runtime
    Runtime,
    /// expansion macro
    ExpandTime,
}

impl Phase {
    pub fn as_u32(self) -> u32 {
        match self {
            Phase::Runtime => 0,
            Phase::ExpandTime => 1,
        }
    }
}
