//! @path: karst/crates/karst_expander/lib.rs
//! @author: redskaber
//! @datetime: 2026-10-04
//! @discription: karst::crates::karst_expander
//!
//! Macro Expandsion: SyntaxObject -> CoreExpr

pub mod ctx;
pub mod error;
pub mod expand;

pub use ctx::{EXPANSION_LIMIT, ExpandCtx};
pub use error::ExpandError;
pub use expand::{expand, expand_program};

pub const STAGE0_GLOBALS: &[&str] = &[
    // op
    "+",
    "-",
    "*",
    "/",
    "<",
    "<=",
    ">",
    ">=",
    "=",
    // pair: (car, cdr), ? => pred "is"
    "car",
    "cdr",
    "cons",
    "pair?",
    "nil?",
    "eq?",
    "not",
    // minimal io
    "read-line",
    "write-line",
];
