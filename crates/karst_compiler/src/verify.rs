//! @path: karst/crates/karst_compiler/verify.rs
//! @author: redskaber
//! @datetime: 2026-10-18
//! @discription: karst::crates::karst_compiler::verify
//!
//! the public product self-check

use std::collections::{BTreeSet, HashMap, VecDeque};

use karst_span::Span;

use crate::{Bytecode, CompileError, Opcode};

impl Bytecode {
    pub fn verify(&self) -> Result<(), CompileError> {
        let Some(_) = self.funcs.first() else {
            return Err(CompileError::EmptyFuncTable);
        };
        for (fi, f) in self.funcs.iter().enumerate() {
            if f.debug_info.len() != f.code.len() {
                return Err(CompileError::DebugInfoMissing {
                    func: fi,
                    pc: f.debug_info.len().min(f.code.len()),
                });
            }
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
                    Opcode::Halt if fi != 0 => {
                        return Err(CompileError::HaltOutsizeEntry { pc, span });
                    }
                    _ => {}
                }
            }
            self.verify_depths(fi)?;
        }
        Ok(())
    }

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
                    }
                    Opcode::Ret => break,
                    Opcode::Halt => {
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

fn propagate(
    depth_at: &mut HashMap<usize, u32>,
    queue: &mut VecDeque<usize>,
    debug_info: &[Span],
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
    // more ...
}
