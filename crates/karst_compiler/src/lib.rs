//! @path: karst/crates/karst_compiler/lib.rs
//! @author: redskaber
//! @datetime: 2026-10-07
//! @discription: karst::crates::karst_compiler
//!
//! bytecode generation from CoreExpr

mod bytecode;
mod compile;
mod error;
mod opcode;
mod verify;

pub use opcode::Opcode;
