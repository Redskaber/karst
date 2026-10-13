//! @path: karst/crates/karst_compiler/error.rs
//! @author: redskaber
//! @datetime: 2026-10-13
//! @discription: karst::crates::karst_compiler::error
//!
//! bytecode generation from CoreExpr - the structured failure surface of bytecode generation

use karst_span::Span;

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
}
