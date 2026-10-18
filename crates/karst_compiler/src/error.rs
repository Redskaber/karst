//! @path: karst/crates/karst_compiler/error.rs
//! @author: redskaber
//! @datetime: 2026-10-13
//! @discription: karst::crates::karst_compiler::error
//!
//! bytecode generation from CoreExpr - the structured failure surface of bytecode generation

use core::fmt;
use std::error::Error;

use karst_span::Span;

use crate::COMPILE_DEPTH_LIMIT;

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    StackUnderflow {
        pc: usize,
        span: Span,
    },
    StackImbalance {
        pc: usize,
        expected: u32,
        actual: u32,
        span: Span,
    },
    DepthExceeded {
        depth: usize,
        span: Span,
    },
    UnpatchedJump {
        at: usize,
        span: Span,
    },
    // verify
    EmptyFuncTable,
    DebugInfoMissing {
        func: usize,
        pc: usize,
    },
    MissingTerminator {
        func: usize,
        pc: usize,
    },
    ConstIndexOutOfRange {
        pc: usize,
        index: usize,
        span: Span,
    },
    GlobalIndexOutOfRange {
        pc: usize,
        index: usize,
        span: Span,
    },
    LocalSlotOutOfRange {
        pc: usize,
        slot: usize,
        span: Span,
    },
    ProtoIndexOutOfRange {
        pc: usize,
        index: usize,
        span: Span,
    },
    JumpTargetOutOfRange {
        pc: usize,
        target: usize,
        span: Span,
    },
    HaltOutsizeEntry {
        pc: usize,
        span: Span,
    },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::DepthExceeded { depth, span } => write!(
                f,
                "compile depth `{depth}` exceeds limit `{}` at {span}",
                COMPILE_DEPTH_LIMIT
            ),
            CompileError::UnpatchedJump { at, span } => {
                write!(f, "unpatched jump placeholder at `{at}` ({span})")
            }
            CompileError::StackUnderflow { pc, span } => {
                write!(f, "stack underflow at pc `{pc}` ({span})")
            }
            CompileError::StackImbalance {
                pc,
                expected,
                actual,
                span,
            } => write!(
                f,
                "stack depth `{actual}` at pc `{pc}` disagress with predecessor depth `{expected}` ({span})"
            ),
            CompileError::ConstIndexOutOfRange { pc, index, span } => write!(
                f,
                "constant index `{index}` out of range at pc `{pc}` ({span})"
            ),
            CompileError::GlobalIndexOutOfRange { pc, index, span } => write!(
                f,
                "global index `{index}` out of range at pc `{pc}` ({span})"
            ),
            CompileError::LocalSlotOutOfRange { pc, slot, span } => {
                write!(f, "local slot `{slot}` out of range at pc `{pc}` ({span})")
            }
            CompileError::ProtoIndexOutOfRange { pc, index, span } => write!(
                f,
                "proto index `{index}` out of range at pc `{pc}` ({span})"
            ),
            CompileError::JumpTargetOutOfRange { pc, target, span } => write!(
                f,
                "jump target `{target}` out of range at pc `{pc}` ({span})"
            ),
            CompileError::DebugInfoMissing { func, pc } => write!(
                f,
                "debug info missing for func `{func}` pc `{pc}` (bare instruction)"
            ),
            CompileError::MissingTerminator { func, pc } => write!(
                f,
                "func `{func}` must end with `ret` or `halt` (last pc `{pc}`)"
            ),
            CompileError::HaltOutsizeEntry { pc, span } => {
                write!(f, "`halt` outside entry proto at pc `{pc}` ({span})")
            }
            CompileError::EmptyFuncTable => {
                write!(f, "bytecode has no function proto (entry proto missing)")
            }
        }
    }
}

impl Error for CompileError {}

#[cfg(test)]
mod tests {
    // more ...
}
