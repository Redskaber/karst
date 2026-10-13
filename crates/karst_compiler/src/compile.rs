//! @path: karst/crates/karst_compiler/compile.rs
//! @author: redskaber
//! @datetime: 2026-10-11
//! @discription: karst::crates::karst_compiler::compile
//!
//! bytecode translation

use std::collections::HashMap;

use karst_core::{CoreExpr, LiteralValue, ModuleItem};
use karst_span::{ByteOffset, ExpansionId, FileId, Span};
use karst_syntax::Symbol;

use crate::{Bytecode, COMPILE_DEPTH_LIMIT, FuncProto, JUMP_PLACEHOLDER};
use crate::{CompileError, Instr, Opcode};

/// Canonical constant-pool key
#[derive(Debug, PartialEq, Eq, Hash)]
enum ConstKey {
    Int(i64),
    Float(u64),
    Bool(bool),
    Str(String),
    Nil,
    Pair(Box<ConstKey>, Box<ConstKey>),
}

fn const_key(value: &LiteralValue) -> ConstKey {
    match value {
        LiteralValue::Int(v) => ConstKey::Int(*v),
        LiteralValue::Float(v) => ConstKey::Float(if v.is_nan() {
            f64::NAN.to_bits()
        } else {
            v.to_bits()
        }),
        LiteralValue::Bool(v) => ConstKey::Bool(*v),
        LiteralValue::String(v) => ConstKey::Str(v.clone()),
        LiteralValue::Nil => ConstKey::Nil,
        LiteralValue::Pair(car, cdr) => {
            ConstKey::Pair(Box::new(const_key(car)), Box::new(const_key(cdr)))
        }
    }
}

/// Per-proto compilation state
struct ProtoState {
    name: Option<Symbol>,
    params: u32,
    n_locals: u32,
    code: Vec<Instr>,
    debug_info: Vec<Span>,
    depth: i32,
    locals: Vec<(Symbol, u32)>,
    jump_sources: HashMap<usize, i32>,
    unpatched: Vec<usize>,
}

struct Complier {
    protos: Vec<ProtoState>,
    cur: usize,
    consts: Vec<LiteralValue>,
    const_map: HashMap<ConstKey, u32>,
    globals: Vec<Symbol>,
    global_map: HashMap<Symbol, u32>,
    depth: usize,
    last_span: Span,
}

impl Complier {
    fn new() -> Self {
        Complier {
            protos: vec![ProtoState {
                name: None,
                params: 0,
                n_locals: 0,
                code: Vec::new(),
                debug_info: Vec::new(),
                depth: 0,
                locals: Vec::new(),
                jump_sources: HashMap::new(),
                unpatched: Vec::new(),
            }],
            cur: 0,
            consts: Vec::new(),
            const_map: HashMap::new(),
            globals: Vec::new(),
            global_map: HashMap::new(),
            depth: 0,
            last_span: Self::anchor(),
        }
    }

    fn anchor() -> Span {
        Span::new(FileId(0), ByteOffset(0), ByteOffset(0), ExpansionId::ROOT)
            .expect("cold-start anchor span is well-formed by construction")
    }

    // --- emission ---

    /// Emit one instruction with automatic debug_info recording and depth bookkeeping
    fn emit(&mut self, op: Opcode, a: u32, b: u32, span: Span) -> Result<usize, CompileError> {
        let p = &mut self.protos[self.cur];
        let pops = op.pops(a, b) as i32;
        let pc = p.code.len();
        if p.depth < pops {
            return Err(CompileError::StackUnderflow { pc, span });
        }
        p.depth = p.depth - pops + op.pushes() as i32;
        p.code.push(Instr { op, a, b });
        p.debug_info.push(span);
        self.last_span = span;

        Ok(pc)
    }

    /// Emit a jump with a placeholder target, return the pc
    fn emit_jump(&mut self, op: Opcode, span: Span) -> Result<usize, CompileError> {
        let pc = self.emit(op, JUMP_PLACEHOLDER, 0, span)?;
        let p = &mut self.protos[self.cur];
        p.jump_sources.insert(pc, p.depth);
        p.unpatched.push(pc);
        Ok(pc)
    }

    /// Patch the placeholder at `at` to `target`
    fn patch(&mut self, at: usize, target: u32) -> Result<(), CompileError> {
        let p = &mut self.protos[self.cur];
        let pos = p.unpatched.iter().position(|&pc| pc == at);
        debug_assert!(
            pos.is_some(),
            "patch of a non-placeholder pc is a compiler bug"
        );
        if let Some(pos) = pos {
            p.unpatched.remove(pos);
        }
        let expected = p.jump_sources[&at];
        let actual = p.depth;
        let span = p.debug_info[at];
        if actual != expected {
            return Err(CompileError::StackImbalance {
                pc: at,
                expected: expected as u32,
                actual: actual as u32,
                span,
            });
        }
        p.code[at].a = target;
        Ok(())
    }

    /// Explicit depth reset
    fn set_depth(&mut self, depth: i32) {
        self.protos[self.cur].depth = depth;
    }

    fn cur_depth(&self) -> i32 {
        self.protos[self.cur].depth
    }

    fn here(&self) -> u32 {
        self.protos[self.cur].code.len() as u32
    }

    /// The tail-position terminator `ret` inside a closure
    fn emit_tail_terminator(&mut self, span: Span) -> Result<(), CompileError> {
        if self.cur == 0 {
            self.emit(Opcode::Halt, 0, 0, span)?;
        } else {
            self.emit(Opcode::Ret, 0, 0, span)?;
        }
        Ok(())
    }

    // --- pools and locals ---

    fn intern_const(&mut self, value: &LiteralValue) -> u32 {
        let key = const_key(value);
        if let Some(&idx) = self.const_map.get(&key) {
            return idx;
        }
        let idx = self.consts.len() as u32;
        self.consts.push(value.clone());
        self.const_map.insert(key, idx);
        idx
    }

    fn intern_global(&mut self, name: &Symbol) -> u32 {
        if let Some(&idx) = self.global_map.get(name) {
            return idx;
        }
        let idx = self.globals.len() as u32;
        self.globals.push(name.clone());
        self.global_map.insert(name.clone(), idx);
        idx
    }

    fn alloc_local(&mut self) -> u32 {
        let p = &mut self.protos[self.cur];
        let slot = p.n_locals;
        p.n_locals += 1;
        slot
    }

    fn push_local(&mut self, name: Symbol, slot: u32) {
        self.protos[self.cur].locals.push((name, slot));
    }

    fn lookup_local(&self, name: &Symbol) -> Option<u32> {
        self.protos[self.cur]
            .locals
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, slot)| *slot)
    }

    fn locals_mark(&self) -> usize {
        self.protos[self.cur].locals.len()
    }

    fn locals_truncate(&mut self, mark: usize) {
        self.protos[self.cur].locals.truncate(mark);
    }

    // --- protos ---

    fn begin_closure(&mut self, name: Option<Symbol>, params: &[Symbol]) -> u32 {
        let idx = self.protos.len() as u32;
        self.protos.push(ProtoState {
            name,
            params: params.len() as u32,
            n_locals: params.len() as u32,
            code: Vec::new(),
            debug_info: Vec::new(),
            depth: 0,
            locals: params.iter().cloned().zip(0u32..).collect(),
            jump_sources: HashMap::new(),
            unpatched: Vec::new(),
        });
        self.cur = idx as usize;
        idx
    }

    fn end_closure(&mut self) {
        self.cur -= 1;
    }

    // --- recursion depth ---

    fn enter_depth(&mut self, span: Span) -> Result<(), CompileError> {
        self.depth += 1;
        if self.depth > COMPILE_DEPTH_LIMIT {
            self.depth -= 1;
            return Err(CompileError::DepthExceeded {
                depth: COMPILE_DEPTH_LIMIT + 1,
                span,
            });
        }
        Ok(())
    }

    fn end_depth(&mut self) {
        self.depth -= 1
    }

    // --- translation ---

    fn expr(&mut self, e: &CoreExpr, tail: bool) -> Result<bool, CompileError> {
        self.enter_depth(e.span())?;
        let result = self.expr_inner(e, tail);
        self.end_depth();
        result
    }

    fn expr_inner(&mut self, e: &CoreExpr, tail: bool) -> Result<bool, CompileError> {
        match e {
            CoreExpr::Literal { span, value } => {
                let k = self.intern_const(value);
                self.emit(Opcode::PushConst, k, 0, *span)?;
                Ok(false)
            }
            CoreExpr::Var { span, name } => {
                if let Some(slot) = self.lookup_local(name) {
                    self.emit(Opcode::LoadLocal, slot, 0, *span)?;
                } else {
                    let g = self.intern_global(name);
                    self.emit(Opcode::LoadGlobal, g, 0, *span)?;
                }
                Ok(false)
            }
            CoreExpr::Fn { span, params, body } => {
                self.compile_fn(None, *span, params, body)?;
                Ok(false)
            }
            CoreExpr::Apply { span, func, args } => {
                for arg in args {
                    self.expr(arg, false)?;
                }
                self.expr(func, false)?;
                self.emit(Opcode::Call, args.len() as u32, 0, *span)?;
                Ok(false)
            }
            CoreExpr::If {
                span,
                cond,
                then_branch,
                else_branch,
            } if tail => {
                self.expr(cond, false)?;
                let jf = self.emit_jump(Opcode::JumpIfFalse, cond.span())?;
                let post_cond = self.cur_depth();
                self.expr(then_branch, false)?;
                self.emit_tail_terminator(then_branch.span())?;
                self.set_depth(post_cond);
                self.patch(jf, self.here())?;
                self.expr(else_branch, false)?;
                self.emit_tail_terminator(else_branch.span())?;
                Ok(true)
            }
            CoreExpr::If {
                span,
                cond,
                then_branch,
                else_branch,
            } => {
                self.expr(cond, false)?;
                let jf = self.emit_jump(Opcode::JumpIfFalse, cond.span())?;
                let post_cond = self.cur_depth();
                self.expr(then_branch, false)?;
                let j_end = self.emit_jump(Opcode::Jump, *span)?;
                self.set_depth(post_cond);
                self.patch(jf, self.here())?;
                self.expr(else_branch, false)?;
                self.patch(j_end, self.here())?;
                Ok(false)
            }
            CoreExpr::Assign { span, name, value } => {
                self.expr(value, false)?;
                self.emit(Opcode::Dup, 0, 0, *span)?;
                if let Some(slot) = self.lookup_local(name) {
                    self.emit(Opcode::StoreLocal, slot, 0, *span)?;
                } else {
                    let g = self.intern_global(name);
                    self.emit(Opcode::StoreGlobal, g, 0, *span)?;
                }
                Ok(false)
            }
            CoreExpr::Let { bindings, body, .. } => {
                let mark = self.locals_mark();
                for binding in bindings {
                    self.expr(&binding.value, false)?;
                    let slot = self.alloc_local();
                    self.emit(Opcode::StoreLocal, slot, 0, binding.span)?;
                    self.push_local(binding.name.clone(), slot);
                }
                let terminated = self.expr(body, tail)?;
                self.locals_truncate(mark);
                Ok(terminated)
            }
            CoreExpr::Perform { span, effect, args } => {
                for arg in args {
                    self.expr(arg, false)?;
                }
                let g = self.intern_global(effect);
                self.emit(Opcode::Perform, g, args.len() as u32, *span)?;
                Ok(false)
            }
            CoreExpr::Handle {
                span,
                bpdy,
                handlers,
            } => {
                for handle in handlers {
                    self.expr(&handle.handler, false)?;
                    let g = self.intern_global(&handle.effect);
                    self.emit(Opcode::PushHandler, g, 0, handle.span)?;
                }
                self.expr(bpdy, false)?;
                self.emit(Opcode::PopHandler, handlers.len() as u32, 0, *span)?;
                Ok(false)
            }
        }
    }

    fn compile_fn(
        &mut self,
        name: Option<&Symbol>,
        span: Span,
        params: &[Symbol],
        body: &CoreExpr,
    ) -> Result<(), CompileError> {
        let idx = self.begin_closure(name.cloned(), params);
        let terminated = self.expr(body, true)?;
        if !terminated {
            self.emit(Opcode::Ret, 0, 0, body.span())?;
        }
        self.end_closure();
        self.emit(Opcode::Closure, idx, 0, span)?;
        Ok(())
    }

    fn item(&mut self, item: &ModuleItem) -> Result<(), CompileError> {
        self.enter_depth(item.span())?;
        let result = self.item_inner(item);
        self.end_depth();
        result
    }

    fn item_inner(&mut self, item: &ModuleItem) -> Result<(), CompileError> {
        match item {
            ModuleItem::Define { span, name, value } => {
                if let CoreExpr::Fn {
                    span: fn_span,
                    params,
                    body,
                } = value
                {
                    self.compile_fn(Some(name), *fn_span, params, body)?;
                } else {
                    self.expr(value, false)?;
                }
                let g = self.intern_global(name);
                self.emit(Opcode::StoreGlobal, g, 0, *span)?;
                Ok(())
            }
            ModuleItem::Require { .. } => Ok(()),
            ModuleItem::Module { items, .. } => {
                for sub in items {
                    self.item(sub)?;
                }
                Ok(())
            }
        }
    }

    fn finish(self) -> Result<Bytecode, CompileError> {
        for p in &self.protos {
            if let Some(&at) = p.unpatched.first() {
                return Err(CompileError::UnpatchedJump {
                    at,
                    span: p.debug_info[at],
                });
            }
        }
        Ok(Bytecode {
            funcs: self
                .protos
                .into_iter()
                .map(|p| FuncProto {
                    name: p.name,
                    params: p.params,
                    n_locals: p.n_locals,
                    code: p.code,
                    debug_info: p.debug_info,
                })
                .collect(),
            counts: self.consts,
            globals: self.globals,
        })
    }
}

pub fn compile(expr: &CoreExpr) -> Result<Bytecode, CompileError> {
    let mut c = Complier::new();
    let terminated = c.expr(expr, true)?;
    if !terminated {
        c.emit(Opcode::Halt, 0, 0, expr.span())?;
    }
    c.finish()
}

pub fn compile_program(items: &[ModuleItem]) -> Result<Bytecode, CompileError> {
    let mut c = Complier::new();
    for item in items {
        c.item(item)?
    }
    let hlat_span = c.last_span;
    c.emit(Opcode::Halt, 0, 0, hlat_span)?;
    c.finish()
}

#[cfg(test)]
mod tests {

    // more ...
}
