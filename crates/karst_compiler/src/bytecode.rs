//! @path: karst/crates/karst_compiler/bytecode.rs
//! @author: redskaber
//! @datetime: 2026-10-11
//! @discription: karst::crates::karst_compiler::bytecode
//!
//! The in-memory compilation product

use std::fmt;

use karst_core::LiteralValue;
use karst_span::Span;
use karst_syntax::Symbol;

use crate::Opcode;

/// opcode plua two u32 operand slots
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Instr {
    pub op: Opcode,
    pub a: u32,
    pub b: u32,
}

impl Instr {
    pub fn new(op: Opcode) -> Self {
        Instr { op, a: 0, b: 0 }
    }
    pub fn with_a(op: Opcode, a: u32) -> Instr {
        Instr { op, a, b: 0 }
    }

    pub fn with_ab(op: Opcode, a: u32, b: u32) -> Instr {
        Instr { op, a, b }
    }
}

impl fmt::Display for Instr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.op.kind_name(), self.a, self.b)
    }
}

/// the entru toplevel protp or closure proto
#[derive(Debug, Clone, PartialEq)]
pub struct FuncProto {
    /// Diagnostic name
    pub name: Option<Symbol>,
    /// Params count
    pub params: u32,
    /// Local slots
    pub n_locals: u32,
    /// Instruction sequence
    pub code: Vec<Instr>,
    /// Per-Instruction source spans
    pub debug_info: Vec<Span>,
}

impl FuncProto {
    pub fn new(name: Option<Symbol>, params: u32, n_locals: u32) -> Self {
        FuncProto {
            name,
            params,
            n_locals,
            code: Vec::new(),
            debug_info: Vec::new(),
        }
    }
}

/// the compilation product
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bytecode {
    pub funcs: Vec<FuncProto>,
    /// Deduplicated constant pool
    pub consts: Vec<LiteralValue>,
    /// Interned global symbol table
    pub globals: Vec<Symbol>,
}

impl Bytecode {
    /// Resolve the source code span of `(func, pc)`
    pub fn span_of(&self, func: usize, pc: usize) -> Option<Span> {
        self.funcs.get(func)?.debug_info.get(pc).copied()
    }

    /// entry proto (funcs[0])
    pub fn entry(&self) -> Option<&FuncProto> {
        self.funcs.first()
    }
}

#[cfg(test)]
mod tests {
    // more ...
}
