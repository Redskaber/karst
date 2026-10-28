//! @path: karst/crates/karst_compiler/opcode
//! @author: redskaber
//! @datetime: 2026-10-07
//! @discription: karst::crates::karst_compiler::opcode
//!
//! bytecode generation from CoreExpr - the bytecode operation set

use std::fmt;

/// One bytecode operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    // stack
    PushConst,
    Pop,
    Dup,
    Swap,
    // function
    Call,
    Ret,
    Closure,
    // control flow
    Jump,
    JumpIfFalse,
    // data construction
    MakePair,
    Car,
    Cdr,
    // variable access
    LoadLocal,
    StoreLocal,
    LoadGlobal,
    StoreGlobal,
    // arithmetic
    Add,
    Sub,
    Mul,
    Div,
    // effects (group extension — primitive carriers)
    PushHandler,
    PopHandler,
    Perform,
    // termination
    Halt,
}

impl Opcode {
    /// All operations, in declaration order
    pub const ALL: [Opcode; 24] = [
        Opcode::PushConst,
        Opcode::Pop,
        Opcode::Dup,
        Opcode::Swap,
        Opcode::Call,
        Opcode::Ret,
        Opcode::Closure,
        Opcode::Jump,
        Opcode::JumpIfFalse,
        Opcode::MakePair,
        Opcode::Car,
        Opcode::Cdr,
        Opcode::LoadLocal,
        Opcode::StoreLocal,
        Opcode::LoadGlobal,
        Opcode::StoreGlobal,
        Opcode::Add,
        Opcode::Sub,
        Opcode::Mul,
        Opcode::Div,
        Opcode::PushHandler,
        Opcode::PopHandler,
        Opcode::Perform,
        Opcode::Halt,
    ];

    /// Stable diagnostic name (lowercase kebab form)
    pub fn kind_name(&self) -> &'static str {
        match self {
            Opcode::PushConst => "push-const",
            Opcode::Pop => "pop",
            Opcode::Dup => "dup",
            Opcode::Swap => "swap",
            Opcode::Call => "call",
            Opcode::Ret => "ret",
            Opcode::Closure => "closure",
            Opcode::Jump => "jump",
            Opcode::JumpIfFalse => "jump-if-false",
            Opcode::MakePair => "make-pair",
            Opcode::Car => "car",
            Opcode::Cdr => "cdr",
            Opcode::LoadLocal => "load-local",
            Opcode::StoreLocal => "store-local",
            Opcode::LoadGlobal => "load-global",
            Opcode::StoreGlobal => "store-global",
            Opcode::Add => "add",
            Opcode::Sub => "sub",
            Opcode::Mul => "mul",
            Opcode::Div => "div",
            Opcode::PushHandler => "push-handler",
            Opcode::PopHandler => "pop-handler",
            Opcode::Perform => "perform",
            Opcode::Halt => "halt",
        }
    }

    /// Whether the opcode ends a basic block
    pub fn is_terminator(&self) -> bool {
        matches!(self, Opcode::Jump | Opcode::Ret | Opcode::Halt)
    }

    /// Declared meaning of the `a`/`b` operand slots
    pub fn operand_help(&self) -> &'static str {
        match self {
            Opcode::PushConst => "a: constant-pool index",
            Opcode::Pop | Opcode::Dup | Opcode::Swap => "none",
            Opcode::Call => "a: argument count",
            Opcode::Ret => "none",
            Opcode::Closure => "a: proto index",
            Opcode::Jump | Opcode::JumpIfFalse => "a: target pc",
            Opcode::MakePair | Opcode::Car | Opcode::Cdr => "none",
            Opcode::LoadLocal | Opcode::StoreLocal => "a: local slot",
            Opcode::LoadGlobal | Opcode::StoreGlobal => "a: global index",
            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div => "none",
            Opcode::PushHandler => "a: effect-name global index",
            Opcode::PopHandler => "a: handler count",
            Opcode::Perform => "a: effect-name global index, b: argument count",
            Opcode::Halt => "none",
        }
    }

    /// Operands the instruction pops off the data stack *before* pushing
    pub fn pops(self, a: u32, b: u32) -> u32 {
        match self {
            Opcode::PushConst | Opcode::Closure | Opcode::LoadLocal | Opcode::LoadGlobal => 0,
            Opcode::Call => a.saturating_add(1),
            Opcode::Perform => b,
            Opcode::Pop
            | Opcode::Dup
            | Opcode::StoreLocal
            | Opcode::StoreGlobal
            | Opcode::JumpIfFalse
            | Opcode::PushHandler
            | Opcode::Ret => 1,
            Opcode::Swap
            | Opcode::MakePair
            | Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div => 2,
            Opcode::Car | Opcode::Cdr => 1,
            Opcode::Jump | Opcode::PopHandler | Opcode::Halt => 0,
        }
    }

    /// Operands the instruction pushes onto the data stack
    pub fn pushes(self) -> u32 {
        match self {
            // dup re-pushes the top: pops 1, pushes 2 (net +1)
            Opcode::Dup => 2,
            Opcode::PushConst
            | Opcode::Closure
            | Opcode::LoadLocal
            | Opcode::LoadGlobal
            | Opcode::Call
            | Opcode::Perform => 1,
            Opcode::Pop
            | Opcode::StoreLocal
            | Opcode::StoreGlobal
            | Opcode::JumpIfFalse
            | Opcode::PushHandler
            | Opcode::Ret
            | Opcode::Jump
            | Opcode::PopHandler
            | Opcode::Halt => 0,
            Opcode::MakePair | Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div => 1,
            Opcode::Swap => 2,
            Opcode::Car | Opcode::Cdr => 1,
        }
    }

    /// net data-stack detla for the instruction
    pub fn stack_delta(self, a: u32, b: u32) -> i32 {
        self.pushes() as i32 - self.pops(a, b) as i32
    }
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.kind_name())
    }
}

#[cfg(test)]
mod tests {
    //! Opcode table tests (FP1/FP2): display names, terminator set,
    //! operand declarations, and the pops/pushes/delta single source.

    use super::*;

    #[test]
    fn all_ops_have_unique_kebab_names() {
        assert_eq!(Opcode::ALL.len(), 24, "opcode count is pinned at 24");
        let mut names: Vec<&str> = Opcode::ALL.iter().map(|op| op.kind_name()).collect();
        names.sort_unstable();
        let len = names.len();
        names.dedup();
        assert_eq!(names.len(), len, "kind names must be unique");
        for name in &names {
            assert!(name.chars().next().is_some_and(char::is_lowercase));
            assert!(!name.ends_with('.'));
        }
        assert_eq!(Opcode::Call.to_string(), "call");
        assert_eq!(Opcode::JumpIfFalse.to_string(), "jump-if-false");
        assert_eq!(Opcode::PushHandler.to_string(), "push-handler");
    }

    #[test]
    fn stack_effect_table_matches_contract() {
        // (op, a, b) -> (pops, pushes, delta) — §8.12 semantics plus the
        // effect extension; the eval-order contract (§19.5) puts the
        // callee on top, so `call a` pops a + 1.
        let cases: [(Opcode, u32, u32, u32, u32); 12] = [
            (Opcode::PushConst, 0, 0, 0, 1),
            (Opcode::Pop, 0, 0, 1, 0),
            (Opcode::Dup, 0, 0, 1, 2),
            (Opcode::Swap, 0, 0, 2, 2),
            (Opcode::Call, 2, 0, 3, 1),
            (Opcode::Ret, 0, 0, 1, 0),
            (Opcode::Closure, 1, 0, 0, 1),
            (Opcode::JumpIfFalse, 5, 0, 1, 0),
            (Opcode::MakePair, 0, 0, 2, 1),
            (Opcode::LoadGlobal, 0, 0, 0, 1),
            (Opcode::PushHandler, 3, 0, 1, 0),
            (Opcode::Perform, 3, 2, 2, 1),
        ];
        for (op, a, b, pops, pushes) in cases {
            assert_eq!(op.pops(a, b), pops, "pops for {op} a={a} b={b}");
            assert_eq!(op.pushes(), pushes, "pushes for {op}");
        }
        // delta spot checks: call arity 2 -> -2; perform argc 2 -> -1;
        // plain value producers -> +1.
        assert_eq!(Opcode::Call.stack_delta(2, 0), -2);
        assert_eq!(Opcode::Perform.stack_delta(3, 2), -1);
        assert_eq!(Opcode::Call.stack_delta(0, 0), 0);
        assert_eq!(Opcode::LoadLocal.stack_delta(0, 0), 1);
        // terminators: only jump / ret / halt.
        for op in Opcode::ALL {
            assert_eq!(
                op.is_terminator(),
                matches!(op, Opcode::Jump | Opcode::Ret | Opcode::Halt),
                "terminator set for {op}"
            );
        }
        // operand_help is declared for every op (no "unknown").
        for op in Opcode::ALL {
            assert!(!op.operand_help().is_empty());
        }
    }
}
