//! @path: karst/crates/karst_compiler/bytecode.rs
//! @author: redskaber
//! @datetime: 2026-10-11
//! @discription: karst::crates::karst_compiler::bytecode
//!
//! The in-memory compilation product. Pure data, hand-constructiable for tests

use std::fmt;

use karst_core::LiteralValue;
use karst_span::Span;
use karst_syntax::Symbol;

use crate::Opcode;

/// One instruction: opcode plua two u32 operand slots
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
    /// Instruction sequence (pc = index)
    pub code: Vec<Instr>,
    /// Per-Instruction source spans, parallel to `code`
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
    //! Bytecode data-shape tests (FP3): constructors, accessors, and
    //! the span_of lookup contract (hit plus both miss directions).

    use super::*;
    use karst_span::{ByteOffset, ExpansionId, FileId};

    fn span(start: u32, end: u32) -> Span {
        Span::new(
            FileId(0),
            ByteOffset(start),
            ByteOffset(end),
            ExpansionId::ROOT,
        )
        .expect("test fixture span must be well-formed")
    }

    fn sample_bytecode() -> Bytecode {
        let mut proto = FuncProto::new(None, 0, 1);
        proto.code = vec![
            Instr::with_a(Opcode::LoadLocal, 0),
            Instr::new(Opcode::Halt),
        ];
        proto.debug_info = vec![span(4, 8), span(4, 8)];
        let mut bc = Bytecode::default();
        bc.funcs.push(proto);
        bc.consts.push(LiteralValue::Int(7));
        bc.globals.push(Symbol::new("+").expect("fixture symbol"));
        bc
    }

    #[test]
    fn instr_builders_and_display_form() {
        let plain = Instr::new(Opcode::Ret);
        assert_eq!(
            plain,
            Instr {
                op: Opcode::Ret,
                a: 0,
                b: 0
            }
        );
        let one = Instr::with_a(Opcode::Call, 2);
        assert_eq!(one.b, 0);
        let both = Instr::with_ab(Opcode::Perform, 3, 2);
        assert_eq!(
            both,
            Instr {
                op: Opcode::Perform,
                a: 3,
                b: 2
            }
        );
        assert_eq!(plain.to_string(), "ret 0 0");
        assert_eq!(one.to_string(), "call 2 0");
        assert_eq!(both.to_string(), "perform 3 2");
    }

    #[test]
    fn proto_and_bytecode_accessors() {
        let bc = sample_bytecode();
        let entry = bc.entry().expect("entry proto present");
        assert!(entry.name.is_none());
        assert_eq!(entry.params, 0);
        assert_eq!(entry.n_locals, 1);
        assert_eq!(entry.code.len(), 2);
        assert_eq!(bc.consts.len(), 1);
        assert_eq!(bc.globals.len(), 1);
        assert_eq!(bc.globals[0].as_str(), "+");
    }

    #[test]
    fn span_of_misses_on_unknown_func() {
        let bc = sample_bytecode();
        assert!(bc.span_of(1, 0).is_none());
    }

    #[test]
    fn span_of_misses_on_out_of_range_pc() {
        let bc = sample_bytecode();
        assert!(bc.span_of(0, 2).is_none());
        assert!(bc.span_of(0, 0).is_some());
    }
}
