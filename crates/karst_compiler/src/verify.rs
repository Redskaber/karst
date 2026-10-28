//! @path: karst/crates/karst_compiler/verify.rs
//! @author: redskaber
//! @datetime: 2026-10-18
//! @discription: karst::crates::karst_compiler::verify
//!
//! the public product self-check

use std::collections::{BTreeSet, HashMap, VecDeque};

use crate::{Bytecode, CompileError, Opcode};

impl Bytecode {
    /// Validate the bytecode: structure and stack-depth consistency
    pub fn verify(&self) -> Result<(), CompileError> {
        let Some(_) = self.funcs.first() else {
            return Err(CompileError::EmptyFuncTable);
        };
        for (fi, f) in self.funcs.iter().enumerate() {
            // Pass 1a: debug_info parallelism (guards span lookups below).
            if f.debug_info.len() != f.code.len() {
                return Err(CompileError::DebugInfoMissing {
                    func: fi,
                    pc: f.debug_info.len().min(f.code.len()),
                });
            }
            // Pass 1b: terminal shape.
            match f.code.last() {
                None => {
                    return Err(CompileError::MissingTerminator { func: fi, pc: 0 });
                }
                Some(last) => {
                    if !last.op.is_terminator() {
                        return Err(CompileError::MissingTerminator {
                            func: fi,
                            pc: f.code.len() - 1,
                        });
                    }
                }
            }
            // Pass 1c: operand bounds and halt position.
            for (pc, instr) in f.code.iter().enumerate() {
                let span = f.debug_info[pc];
                let a = instr.a as usize;
                match instr.op {
                    Opcode::PushConst => {
                        if a >= self.consts.len() {
                            return Err(CompileError::ConstIndexOutOfRange { pc, index: a, span });
                        }
                    }
                    Opcode::LoadGlobal
                    | Opcode::StoreGlobal
                    | Opcode::PushHandler
                    | Opcode::Perform => {
                        if a >= self.globals.len() {
                            return Err(CompileError::GlobalIndexOutOfRange { pc, index: a, span });
                        }
                    }
                    Opcode::LoadLocal | Opcode::StoreLocal => {
                        if instr.a >= f.n_locals {
                            return Err(CompileError::LocalSlotOutOfRange { pc, slot: a, span });
                        }
                    }
                    Opcode::Closure => {
                        if a >= self.funcs.len() {
                            return Err(CompileError::ProtoIndexOutOfRange { pc, index: a, span });
                        }
                    }
                    Opcode::Jump | Opcode::JumpIfFalse => {
                        if a >= f.code.len() {
                            return Err(CompileError::JumpTargetOutOfRange {
                                pc,
                                target: a,
                                span,
                            });
                        }
                    }
                    // halt is the entry terminator (funcs[0]) only
                    Opcode::Halt if fi != 0 => {
                        return Err(CompileError::HaltOutsideEntry { pc, span });
                    }
                    _ => {}
                }
            }
            // Pass 2: stack-depth propagation over basic blocks.
            self.verify_depths(fi)?;
        }
        Ok(())
    }

    /// The DAG depth walk for one proto
    fn verify_depths(&self, fi: usize) -> Result<(), CompileError> {
        let f = &self.funcs[fi];
        let n = f.code.len();
        let mut leaders: BTreeSet<usize> = BTreeSet::new();
        leaders.insert(0);
        for (pc, instr) in f.code.iter().enumerate() {
            match instr.op {
                Opcode::Jump | Opcode::JumpIfFalse => {
                    leaders.insert(instr.a as usize);
                }
                _ => {}
            }
            if instr.op.is_terminator() && pc < n {
                leaders.insert(pc + 1);
            }
        }
        leaders.retain(|&pc| pc < n);

        let mut depth_at: HashMap<usize, u32> = HashMap::new();
        let mut queue: VecDeque<usize> = VecDeque::new();
        depth_at.insert(0, 0);
        queue.push_back(0);

        while let Some(start) = queue.pop_front() {
            let mut depth = depth_at[&start];
            let mut pc = start;
            while pc < n {
                let instr = f.code[pc];
                let span = f.debug_info[pc];
                let pops = instr.op.pops(instr.a, instr.b);
                if depth < pops {
                    return Err(CompileError::StackUnderflow { pc, span });
                }
                depth = depth - pops + instr.op.pushes();
                match instr.op {
                    Opcode::Jump => {
                        propagate(
                            &mut depth_at,
                            &mut queue,
                            &f.debug_info,
                            instr.a as usize,
                            depth,
                        )?;
                        break;
                    }
                    Opcode::JumpIfFalse => {
                        propagate(
                            &mut depth_at,
                            &mut queue,
                            &f.debug_info,
                            instr.a as usize,
                            depth,
                        )?;
                        // fallthrough continues at the same depth
                    }
                    Opcode::Ret => break,
                    Opcode::Halt => {
                        // The entry terminator returns the top of the stack
                        if depth > 1 {
                            return Err(CompileError::StackImbalance {
                                pc,
                                expected: 1,
                                actual: depth,
                                span,
                            });
                        }
                        break;
                    }
                    _ => {}
                }
                pc += 1;
                if pc < n && leaders.contains(&pc) {
                    propagate(&mut depth_at, &mut queue, &f.debug_info, pc, depth)?;
                    break;
                }
            }
        }
        Ok(())
    }
}

/// Propagate `depth` to the block starting at `target`
fn propagate(
    depth_at: &mut HashMap<usize, u32>,
    queue: &mut VecDeque<usize>,
    debug_info: &[karst_span::Span],
    target: usize,
    depth: u32,
) -> Result<(), CompileError> {
    match depth_at.get(&target) {
        Some(&recorded) => {
            if recorded != depth {
                return Err(CompileError::StackImbalance {
                    pc: target,
                    expected: recorded,
                    actual: depth,
                    span: debug_info[target],
                });
            }
        }
        None => {
            depth_at.insert(target, depth);
            queue.push_back(target);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! verify rejection tests (FP1/FP4): one hand-built bad product per
    //! defense variant — the public negative face

    use super::*;
    use crate::bytecode::{FuncProto, Instr};
    use crate::opcode::Opcode;
    use karst_span::{ByteOffset, ExpansionId, FileId, Span};

    fn span() -> Span {
        Span::new(FileId(0), ByteOffset(0), ByteOffset(1), ExpansionId::ROOT)
            .expect("test fixture span must be well-formed")
    }

    /// Minimal legal entry proto with one pooled constant (index 0).
    fn entry_of(code: Vec<Instr>, debug_len: usize, n_locals: u32) -> Bytecode {
        let mut f = FuncProto::new(None, 0, n_locals);
        f.code = code;
        f.debug_info = vec![span(); debug_len];
        let mut bc = Bytecode::default();
        bc.consts.push(karst_core::LiteralValue::Int(0));
        bc.funcs.push(f);
        bc
    }

    #[test]
    fn verify_rejects_empty_func_table() {
        assert_eq!(
            Bytecode::default().verify().unwrap_err(),
            CompileError::EmptyFuncTable
        );
    }

    #[test]
    fn verify_rejects_bare_instruction_missing_debug_info() {
        let bc = entry_of(vec![Instr::new(Opcode::Halt)], 0, 0);
        assert_eq!(
            bc.verify().unwrap_err(),
            CompileError::DebugInfoMissing { func: 0, pc: 0 }
        );
    }

    #[test]
    fn verify_rejects_extra_debug_info() {
        let bc = entry_of(vec![Instr::new(Opcode::Halt)], 2, 0);
        assert_eq!(
            bc.verify().unwrap_err(),
            CompileError::DebugInfoMissing { func: 0, pc: 1 }
        );
    }

    #[test]
    fn verify_rejects_missing_terminator_and_empty_proto() {
        let bad_tail = entry_of(vec![Instr::new(Opcode::Pop)], 1, 0);
        assert_eq!(
            bad_tail.verify().unwrap_err(),
            CompileError::MissingTerminator { func: 0, pc: 0 }
        );
        let empty = entry_of(vec![], 0, 0);
        assert_eq!(
            empty.verify().unwrap_err(),
            CompileError::MissingTerminator { func: 0, pc: 0 }
        );
    }

    #[test]
    fn verify_rejects_const_index_out_of_range() {
        let bc = entry_of(
            vec![
                Instr::with_a(Opcode::PushConst, 1),
                Instr::new(Opcode::Halt),
            ],
            2,
            0,
        );
        // the helper pools one constant: index 1 is out of range.
        assert!(matches!(
            bc.verify().unwrap_err(),
            CompileError::ConstIndexOutOfRange {
                pc: 0,
                index: 1,
                ..
            }
        ));
    }

    #[test]
    fn verify_rejects_global_index_out_of_range() {
        let bc = entry_of(
            vec![
                Instr::with_a(Opcode::LoadGlobal, 0),
                Instr::new(Opcode::Halt),
            ],
            2,
            0,
        );
        assert!(matches!(
            bc.verify().unwrap_err(),
            CompileError::GlobalIndexOutOfRange {
                pc: 0,
                index: 0,
                ..
            }
        ));
    }

    #[test]
    fn verify_rejects_local_slot_out_of_range() {
        let bc = entry_of(
            vec![
                Instr::with_a(Opcode::LoadLocal, 0),
                Instr::new(Opcode::Halt),
            ],
            2,
            0,
        );
        // n_locals = 0: slot 0 is out of range.
        assert!(matches!(
            bc.verify().unwrap_err(),
            CompileError::LocalSlotOutOfRange { pc: 0, slot: 0, .. }
        ));
    }

    #[test]
    fn verify_rejects_proto_index_out_of_range() {
        let bc = entry_of(
            vec![Instr::with_a(Opcode::Closure, 1), Instr::new(Opcode::Halt)],
            2,
            0,
        );
        assert!(matches!(
            bc.verify().unwrap_err(),
            CompileError::ProtoIndexOutOfRange {
                pc: 0,
                index: 1,
                ..
            }
        ));
    }

    #[test]
    fn verify_rejects_jump_target_out_of_range() {
        let bc = entry_of(
            vec![
                Instr::new(Opcode::PushConst),
                Instr::with_a(Opcode::JumpIfFalse, 3),
                Instr::new(Opcode::Halt),
            ],
            3,
            0,
        );
        // target 3 == code.len() is out of range.
        assert!(matches!(
            bc.verify().unwrap_err(),
            CompileError::JumpTargetOutOfRange {
                pc: 1,
                target: 3,
                ..
            }
        ));
    }
    #[test]
    fn verify_rejects_stack_underflow() {
        // pop on an empty stack at depth 0.
        let bc = entry_of(
            vec![Instr::new(Opcode::Pop), Instr::new(Opcode::Halt)],
            2,
            0,
        );
        assert!(matches!(
            bc.verify().unwrap_err(),
            CompileError::StackUnderflow { pc: 0, .. }
        ));
    }

    #[test]
    fn verify_rejects_branch_depth_imbalance() {
        // Two pushes, a conditional jump over one more push: the
        // target (pc 4) is recorded at depth 1 while the fallthrough
        // arrives at depth 2.
        let bc = entry_of(
            vec![
                Instr::new(Opcode::PushConst),         // pc 0 -> depth 1
                Instr::new(Opcode::PushConst),         // pc 1 -> depth 2
                Instr::with_a(Opcode::JumpIfFalse, 4), // pc 2 -> depth 1, target 4 at depth 1
                Instr::new(Opcode::PushConst),         // pc 3 -> depth 2
                Instr::new(Opcode::Halt), // pc 4: fallthrough depth 2 vs target depth 1
            ],
            5,
            0,
        );
        assert!(matches!(
            bc.verify().unwrap_err(),
            CompileError::StackImbalance {
                pc: 4,
                expected: 1,
                actual: 2,
                ..
            }
        ));
    }
}
