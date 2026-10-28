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
    /// Linear data-stack depth bookkeeping.
    depth: i32,
    /// Lexical local bindings (stack discipline: mark/truncate).
    locals: Vec<(Symbol, u32)>,
    /// (pc -> depth at the jump source) for patch assertions.
    jump_sources: HashMap<usize, i32>,
    /// pcs of placeholders not yet patched.
    unpatched: Vec<usize>,
}

struct Compiler {
    protos: Vec<ProtoState>,
    cur: usize,
    consts: Vec<LiteralValue>,
    const_map: HashMap<ConstKey, u32>,
    globals: Vec<Symbol>,
    global_map: HashMap<Symbol, u32>,
    /// Current compile-recursion depth
    depth: usize,
    /// Span of the last emit
    last_span: Span,
}

impl Compiler {
    fn new() -> Self {
        Compiler {
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

    /// Cold-start anchor: a real (zero-length) span at origin.
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

    /// Compile one expression. Returns `true` when the expression has
    /// already emitted its own tail terminator (a tail `If`): the
    /// caller must not emit another one, and the proto-level depth has
    /// already been consumed.
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
                cond,
                then_branch,
                else_branch,
                ..
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
                body,
                handlers,
            } => {
                // handler values are evaluated at installation (any
                // CoreExpr; callability is a 0.6 VM check), then the
                // body runs non-tail (handlers pop before return —
                // plan-0.5.md decision 8).
                for clause in handlers {
                    self.expr(&clause.handler, false)?;
                    let g = self.intern_global(&clause.effect);
                    self.emit(Opcode::PushHandler, g, 0, clause.span)?;
                }
                self.expr(body, false)?;
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
            ModuleItem::Require { .. } => {
                // Zero code: linking is deferred to VM load time
                Ok(())
            }
            ModuleItem::Module { items, .. } => {
                // Recursive flat compilation
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
            consts: self.consts,
            globals: self.globals,
        })
    }
}

/// Compile a single expression: the entry proto leaves the value on
/// the stack and `halt`s (depth-1 halt form).
pub fn compile(expr: &CoreExpr) -> Result<Bytecode, CompileError> {
    let mut c = Compiler::new();
    let terminated = c.expr(expr, true)?;
    if !terminated {
        c.emit(Opcode::Halt, 0, 0, expr.span())?;
    }
    c.finish()
}

/// Compile a program (declaration sequence): every `define` nets zero
/// on the stack; the entry proto ends with a depth-0 `halt`.
pub fn compile_program(items: &[ModuleItem]) -> Result<Bytecode, CompileError> {
    let mut c = Compiler::new();
    for item in items {
        c.item(item)?;
    }
    let halt_span = c.last_span;
    c.emit(Opcode::Halt, 0, 0, halt_span)?;
    c.finish()
}

#[cfg(test)]
mod tests {
    //! Translation tests (FP2/FP3): per-primitive instruction shapes,
    //! the tail-If contract, pool dedup, and the compile-time defense
    //! paths (depth cap, unpatched placeholder, patch assertion).

    use super::*;
    use crate::bytecode::Bytecode;
    use karst_span::{ByteOffset as B, ExpansionId, FileId as F};

    fn s(start: u32, end: u32) -> Span {
        Span::new(F(0), B(start), B(end), ExpansionId::ROOT).expect("fixture span")
    }

    fn lit(v: i64) -> CoreExpr {
        CoreExpr::Literal {
            span: s(0, 2),
            value: LiteralValue::Int(v),
        }
    }

    fn var(name: &str) -> CoreExpr {
        CoreExpr::var(s(0, 1), name).unwrap()
    }

    fn ops(bc: &Bytecode, func: usize) -> Vec<Opcode> {
        bc.funcs[func].code.iter().map(|i| i.op).collect()
    }

    #[test]
    fn literal_var_forms_and_const_pool_dedup() {
        // (f 1 1 2): value forms plus pool dedup in one program.
        let program = CoreExpr::Apply {
            span: s(0, 8),
            func: Box::new(var("f")),
            args: vec![lit(1), lit(1), lit(2)],
        };
        let bc = compile(&program).unwrap();
        assert_eq!(
            bc.consts.len(),
            2,
            "duplicate int literals share one pool slot"
        );
        assert_eq!(bc.consts[0], LiteralValue::Int(1));
        assert_eq!(bc.consts[1], LiteralValue::Int(2));
        // NaN folding: distinct NaN payloads collapse to one slot.
        let nan_a = CoreExpr::Literal {
            span: s(0, 3),
            value: LiteralValue::Float(f64::NAN),
        };
        let nan_b = CoreExpr::Literal {
            span: s(0, 3),
            value: LiteralValue::Float(f64::NAN.copysign(-1.0)),
        };
        let both = CoreExpr::Apply {
            span: s(0, 6),
            func: Box::new(var("g")),
            args: vec![nan_a, nan_b],
        };
        let bc2 = compile(&both).unwrap();
        assert_eq!(
            bc2.consts.len(),
            1,
            "all NaNs fold into a single canonical slot"
        );
        // var: global form when no local is in scope; call arity.
        assert_eq!(
            ops(&bc, 0),
            vec![
                Opcode::PushConst,
                Opcode::PushConst,
                Opcode::PushConst,
                Opcode::LoadGlobal,
                Opcode::Call,
                Opcode::Halt,
            ]
        );
        assert_eq!(bc.funcs[0].code[3].a, 0, "first global intern gets slot 0");
        assert_eq!(bc.funcs[0].code[4].a, 3, "call arity counts the args");
        assert!(bc.verify().is_ok());
    }

    #[test]
    fn fn_closure_apply_and_eval_order() {
        // (fn (x) (f x 7)): closure proto, then apply with args before
        // the callee (eval-order contract).
        let body = CoreExpr::Apply {
            span: s(4, 12),
            func: Box::new(var("f")),
            args: vec![var("x"), lit(7)],
        };
        let f = CoreExpr::fn_(s(0, 12), &["x"], body).unwrap();
        let bc = compile(&f).unwrap();
        assert_eq!(bc.funcs.len(), 2, "entry + one closure proto");
        assert_eq!(bc.funcs[1].params, 1);
        assert_eq!(bc.funcs[1].n_locals, 1, "params occupy local slots");
        // entry: closure + halt; closure: load-local x, push 7,
        // load-global f, call 2, ret.
        assert_eq!(ops(&bc, 0), vec![Opcode::Closure, Opcode::Halt]);
        assert_eq!(
            ops(&bc, 1),
            vec![
                Opcode::LoadLocal,
                Opcode::PushConst,
                Opcode::LoadGlobal,
                Opcode::Call,
                Opcode::Ret,
            ]
        );
        assert_eq!(bc.funcs[1].code[0].a, 0, "param x lives in slot 0");
        assert_eq!(bc.funcs[1].code[3].a, 2);
        assert!(bc.verify().is_ok(), "compile output always passes verify");
    }

    #[test]
    fn if_forms_tail_and_non_tail() {
        // Non-tail if (as an argument): jump-if-false + merge jump.
        let call = CoreExpr::Apply {
            span: s(0, 10),
            func: Box::new(var("g")),
            args: vec![CoreExpr::If {
                span: s(1, 9),
                cond: Box::new(var("c")),
                then_branch: Box::new(lit(1)),
                else_branch: Box::new(lit(2)),
            }],
        };
        let bc = compile(&call).unwrap();
        assert_eq!(
            ops(&bc, 0),
            vec![
                Opcode::LoadGlobal, // c
                Opcode::JumpIfFalse,
                Opcode::PushConst,  // 1
                Opcode::Jump,       // merge
                Opcode::PushConst,  // 2
                Opcode::LoadGlobal, // g
                Opcode::Call,
                Opcode::Halt,
            ]
        );
        let jf_target = bc.funcs[0].code[1].a as usize;
        let j_target = bc.funcs[0].code[3].a as usize;
        assert_eq!(
            jf_target, 4,
            "else entry is the instruction after the merge jump"
        );
        assert_eq!(j_target, 5, "merge point is right after else");
        // Tail if (fn body): branches end in ret, no merge jump.
        let f = CoreExpr::fn_(
            s(0, 12),
            &["x"],
            CoreExpr::If {
                span: s(8, 12),
                cond: Box::new(var("x")),
                then_branch: Box::new(lit(1)),
                else_branch: Box::new(lit(2)),
            },
        )
        .unwrap();
        let bc2 = compile(&f).unwrap();
        assert_eq!(
            ops(&bc2, 1),
            vec![
                Opcode::LoadLocal,
                Opcode::JumpIfFalse,
                Opcode::PushConst,
                Opcode::Ret, // tail terminator, then-branch
                Opcode::PushConst,
                Opcode::Ret, // explicit fn-body ret (else falls through)
            ],
            "tail if has no merge jump — branches go straight to ret"
        );
        assert_eq!(
            bc2.funcs[1].code[1].a as usize, 4,
            "else entry after then's ret"
        );
        assert!(bc2.verify().is_ok());
    }

    #[test]
    fn let_assign_perform_handle_forms() {
        // (let ((x 1) (y 2)) (set! x (perform read))) — let chain,
        // assign dup+store, perform with argc 0.
        let perform = CoreExpr::perform(s(9, 21), "read", vec![]).unwrap();
        let assign = CoreExpr::assign(s(9, 21), "x", perform).unwrap();
        let letform = CoreExpr::Let {
            span: s(0, 22),
            bindings: vec![
                karst_core::Binding::new(s(6, 7), "x", lit(1)).unwrap(),
                karst_core::Binding::new(s(7, 8), "y", lit(2)).unwrap(),
            ],
            body: Box::new(assign),
        };
        let bc = compile(&letform).unwrap();
        assert_eq!(
            ops(&bc, 0),
            vec![
                Opcode::PushConst,  // 1
                Opcode::StoreLocal, // x -> slot 0
                Opcode::PushConst,  // 2
                Opcode::StoreLocal, // y -> slot 1
                Opcode::Perform,    // read, argc 0
                Opcode::Dup,
                Opcode::StoreLocal, // assign x
                Opcode::Halt,
            ]
        );
        assert_eq!(bc.funcs[0].n_locals, 2);
        assert_eq!(bc.funcs[0].code[1].a, 0);
        assert_eq!(bc.funcs[0].code[3].a, 1);
        assert_eq!(bc.funcs[0].code[6].a, 0, "assign resolves innermost x");
        assert_eq!(
            bc.funcs[0].code[4].a, 0,
            "effect name interns in the global table"
        );
        assert_eq!(bc.globals[0].as_str(), "read");
        assert!(bc.verify().is_ok());
        // handle: two clauses -> push-handler x2, body, pop-handler 2.
        let handled = CoreExpr::handle(
            s(0, 10),
            var("body-val"),
            vec![
                karst_core::HandlerClause::new(s(2, 4), "read", var("h1")).unwrap(),
                karst_core::HandlerClause::new(s(4, 6), "write", var("h2")).unwrap(),
            ],
        )
        .unwrap();
        let bc2 = compile(&handled).unwrap();
        assert_eq!(
            ops(&bc2, 0),
            vec![
                Opcode::LoadGlobal, // h1
                Opcode::PushHandler,
                Opcode::LoadGlobal, // h2
                Opcode::PushHandler,
                Opcode::LoadGlobal, // body
                Opcode::PopHandler,
                Opcode::Halt,
            ]
        );
        assert_eq!(bc2.funcs[0].code[5].a, 2, "pop-handler counts both clauses");
        // Intern order follows evaluation order: handler values first,
        // then the effect name of each clause.
        assert_eq!(bc2.globals[0].as_str(), "h1");
        assert_eq!(bc2.globals[1].as_str(), "read");
        assert_eq!(bc2.globals[2].as_str(), "h2");
        assert_eq!(bc2.globals[3].as_str(), "write");
        assert!(bc2.verify().is_ok());
    }

    #[test]
    fn depth_cap_rejects_synthesized_deep_tree_and_boundary_passes() {
        fn deep_apply(n: usize) -> CoreExpr {
            let mut e = lit(42);
            for _ in 0..n {
                e = CoreExpr::Apply {
                    span: s(0, 1),
                    func: Box::new(var("f")),
                    args: vec![e],
                };
            }
            e
        }
        // n nested applies -> recursion depth n + 1 (applies + inner
        // literal): n = 1023 peaks exactly at the limit and passes.
        let ok = compile(&deep_apply(COMPILE_DEPTH_LIMIT - 1)).unwrap();
        assert!(ok.verify().is_ok());
        // n = 1024 peaks one over the cap: structured rejection.
        let err = compile(&deep_apply(COMPILE_DEPTH_LIMIT)).unwrap_err();
        assert!(matches!(
            err,
            CompileError::DepthExceeded { depth: 1025, .. }
        ));
    }

    #[test]
    fn codebuf_unpatched_jump_defense_fires_at_finish() {
        // Internal defense: a placeholder that never got patched must
        // surface as a structured error at finish, not as a bad
        // product (the placeholder sentinel would fail verify as an
        // out-of-range target — this defense fires first, at the
        // producer).
        let mut c = Compiler::new();
        c.expr(&lit(1), false).unwrap();
        let jf = c.emit_jump(Opcode::JumpIfFalse, s(0, 1)).unwrap();
        assert!(c.protos[0].unpatched.contains(&jf));
        c.emit(Opcode::Halt, 0, 0, s(0, 1)).unwrap();
        let err = c.finish().unwrap_err();
        assert!(matches!(err, CompileError::UnpatchedJump { at, .. } if at == jf));
    }

    #[test]
    fn codebuf_patch_depth_assertion_defense() {
        // Internal defense: patching with a mismatched current depth
        // (the else-entry reset was skipped) is a bookkeeping bug and
        // must fail closed instead of emitting a skewed target.
        let mut c = Compiler::new();
        c.expr(&var("c"), false).unwrap();
        let jf = c.emit_jump(Opcode::JumpIfFalse, s(0, 1)).unwrap();
        // then-branch leaves one extra value on the linear depth;
        // forget the reset and patch directly.
        c.expr(&lit(1), false).unwrap();
        let err = c.patch(jf, c.here()).unwrap_err();
        assert!(matches!(
            err,
            CompileError::StackImbalance { pc, expected: 0, actual: 1, .. } if pc == jf
        ));
    }
}
