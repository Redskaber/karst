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

pub use bytecode::{Bytecode, FuncProto, Instr};
pub use compile::{compile, compile_program};
pub use error::CompileError;
pub use opcode::Opcode;

/// Jump placeholder sentinel emitted by `emit_jump`
pub const JUMP_PLACEHOLDER: u32 = u32::MAX;
/// Compile-recursion depth cap
pub const COMPILE_DEPTH_LIMIT: usize = 1024;
