//! @path: karst/crates/karst_vm/frame.rs
//! @author: redskaber
//! @datetime: 2026-11-02
//! @discription: karst::crates::karst_vm::frame
//!
//! bytecode virtual machine - call frames

use karst_syntax::Symbol;

use crate::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuationSlot {
    Reserved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionTableSlot {
    Reserved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugFrameSlot {
    pub func_name: Option<Symbol>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameExt {
    pub continuation: ContinuationSlot,
    pub exception_table: ExceptionTableSlot,
    pub debug: DebugFrameSlot,
}

impl FrameExt {
    pub fn new(func_name: Option<Symbol>) -> Self {
        FrameExt {
            continuation: ContinuationSlot::Reserved,
            exception_table: ExceptionTableSlot::Reserved,
            debug: DebugFrameSlot { func_name },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub func: usize,
    pub pc: usize,
    pub locals: Vec<Value>,
    pub stack: Vec<Value>,
    pub ext: FrameExt,
}

impl Frame {
    pub fn new(func: usize, func_name: Option<Symbol>, n_locals: u32, args: Vec<Value>) -> Frame {
        let mut locals = vec![Value::Nil; n_locals as usize];
        if args.len() > locals.len() {
            locals.resize(args.len(), Value::Nil);
        }
        for (slot, arg) in locals.iter_mut().zip(args) {
            *slot = arg;
        }
        Frame {
            func,
            pc: 0,
            locals,
            stack: Vec::new(),
            ext: FrameExt::new(func_name),
        }
    }
}

#[cfg(test)]
mod tests {

    // more ...
}
