//! Crate-public API tests for karst-compiler

use karst_compiler::{Bytecode, CompileError, FuncProto, Instr, Opcode, compile, compile_program};
use karst_core::{CoreExpr, LiteralValue, ModuleItem};
use karst_span::{ByteOffset, ExpansionId, FileId, Span};

fn span(start: u32, end: u32) -> Span {
    Span::new(
        FileId(0),
        ByteOffset(start),
        ByteOffset(end),
        ExpansionId::ROOT,
    )
    .expect("test fixture span must be well-formed")
}

/// `(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))`
fn fib_items() -> Vec<ModuleItem> {
    let body = CoreExpr::If {
        span: span(16, 50),
        cond: CoreExpr::Apply {
            span: span(20, 27),
            func: Box::new(CoreExpr::var(span(20, 21), "<").unwrap()),
            args: vec![
                CoreExpr::var(span(24, 25), "n").unwrap(),
                CoreExpr::Literal {
                    span: span(26, 27),
                    value: LiteralValue::Int(2),
                },
            ],
        }
        .into(),
        then_branch: Box::new(CoreExpr::var(span(29, 30), "n").unwrap()),
        else_branch: Box::new(CoreExpr::Apply {
            span: span(32, 49),
            func: Box::new(CoreExpr::var(span(33, 34), "+").unwrap()),
            args: vec![
                CoreExpr::Apply {
                    span: span(36, 46),
                    func: Box::new(CoreExpr::var(span(36, 39), "fib").unwrap()),
                    args: vec![CoreExpr::Apply {
                        span: span(40, 45),
                        func: Box::new(CoreExpr::var(span(40, 41), "-").unwrap()),
                        args: vec![
                            CoreExpr::var(span(42, 43), "n").unwrap(),
                            CoreExpr::Literal {
                                span: span(44, 45),
                                value: LiteralValue::Int(1),
                            },
                        ],
                    }],
                },
                CoreExpr::Apply {
                    span: span(47, 48),
                    func: Box::new(CoreExpr::var(span(48, 49), "-").unwrap()),
                    args: vec![
                        CoreExpr::var(span(48, 49), "n").unwrap(),
                        CoreExpr::Literal {
                            span: span(48, 49),
                            value: LiteralValue::Int(2),
                        },
                    ],
                },
            ],
        }),
    };
    let fib_fn = CoreExpr::fn_(span(0, 51), &["n"], body).unwrap();
    vec![ModuleItem::define(span(0, 51), "fib", fib_fn).unwrap()]
}

fn bad_entry(code: Vec<Instr>, debug_len: usize) -> Bytecode {
    let mut f = FuncProto::new(None, 0, 0);
    f.code = code;
    f.debug_info = vec![span(0, 1); debug_len];
    let mut bc = Bytecode::default();
    bc.consts.push(LiteralValue::Int(0));
    bc.funcs.push(f);
    bc
}

// ---------- positive ----------

#[test]
fn fib_program_compiles_verifies_and_maps_spans() {
    let bc = compile_program(&fib_items()).unwrap();
    // entry: closure(fib) + store-global fib + halt
    let entry = bc.entry().expect("entry proto");
    assert_eq!(entry.code.len(), 3);
    assert_eq!(entry.code[0].op, Opcode::Closure);
    assert_eq!(entry.code[1].op, Opcode::StoreGlobal);
    assert_eq!(entry.code[2].op, Opcode::Halt);
    assert_eq!(bc.globals.len(), 4, "< - fib + interned by name (dedup)");
    assert!(bc.globals.iter().any(|s| s.as_str() == "fib"));
    // closure proto: named fib, one param, tail if without merge jump
    let fib = &bc.funcs[1];
    assert_eq!(fib.name.as_ref().map(|s| s.as_str()), Some("fib"));
    assert_eq!(fib.params, 1);
    let has_call2 = fib.code.iter().any(|i| i.op == Opcode::Call && i.a == 2);
    assert!(has_call2, "recursive calls carry arity 2 (callee + n)");
    assert!(fib.code.iter().any(|i| i.op == Opcode::JumpIfFalse));
    assert_eq!(fib.code.last().unwrap().op, Opcode::Ret);
    // contract: compile output always passes verify
    bc.verify().unwrap();
    // span_of resolves through debug_info
    let s = bc.span_of(0, 0).expect("span at entry pc 0");
    assert_eq!(s, span(0, 51));
    assert!(bc.span_of(0, 99).is_none());
}

#[test]
fn compile_single_expression_ends_in_depth_one_halt() {
    // (car (cons 1 nil)) — plain value form; the entry leaves exactly
    // one value for halt to return.
    let expr = CoreExpr::Apply {
        span: span(0, 16),
        func: Box::new(CoreExpr::var(span(0, 3), "car").unwrap()),
        args: vec![CoreExpr::Apply {
            span: span(5, 15),
            func: Box::new(CoreExpr::var(span(5, 9), "cons").unwrap()),
            args: vec![
                CoreExpr::Literal {
                    span: span(10, 11),
                    value: LiteralValue::Int(1),
                },
                CoreExpr::Literal {
                    span: span(13, 16),
                    value: LiteralValue::Nil,
                },
            ],
        }],
    };
    let bc = compile(&expr).unwrap();
    bc.verify().unwrap();
    assert_eq!(bc.entry().unwrap().code.last().unwrap().op, Opcode::Halt);
    // args left-to-right then callee: 1, nil, cons, call 1, car, call 1
    let ops: Vec<Opcode> = bc.entry().unwrap().code.iter().map(|i| i.op).collect();
    assert_eq!(
        ops,
        vec![
            Opcode::PushConst,
            Opcode::PushConst,
            Opcode::LoadGlobal,
            Opcode::Call,
            Opcode::LoadGlobal,
            Opcode::Call,
            Opcode::Halt,
        ]
    );
}

// ---------- negative ----------

#[test]
fn depth_cap_rejects_synthesized_chain_over_the_limit() {
    let mut e = CoreExpr::Literal {
        span: span(0, 2),
        value: LiteralValue::Int(0),
    };
    for _ in 0..karst_compiler::COMPILE_DEPTH_LIMIT {
        e = CoreExpr::Apply {
            span: span(0, 1),
            func: Box::new(CoreExpr::var(span(0, 1), "f").unwrap()),
            args: vec![e],
        };
    }
    let err = compile(&e).unwrap_err();
    assert!(matches!(
        err,
        CompileError::DepthExceeded { depth: 1025, .. }
    ));
    // the error message keeps the R8 shape (lowercase, no period)
    let msg = err.to_string();
    assert!(msg.starts_with("compile depth `1025` exceeds limit `1024`"));
    assert!(!msg.ends_with('.'));
}

#[test]
fn verify_rejects_jump_target_out_of_range_product() {
    let bc = bad_entry(
        vec![
            Instr::new(Opcode::PushConst),
            Instr::with_a(Opcode::Jump, 9),
            Instr::new(Opcode::Halt),
        ],
        3,
    );
    assert!(matches!(
        bc.verify().unwrap_err(),
        CompileError::JumpTargetOutOfRange {
            pc: 1,
            target: 9,
            ..
        }
    ));
}

#[test]
fn verify_rejects_stack_underflow_product() {
    let bc = bad_entry(vec![Instr::new(Opcode::Pop), Instr::new(Opcode::Halt)], 2);
    assert!(matches!(
        bc.verify().unwrap_err(),
        CompileError::StackUnderflow { pc: 0, .. }
    ));
}

#[test]
fn verify_rejects_branch_imbalance_product() {
    let bc = bad_entry(
        vec![
            Instr::new(Opcode::PushConst),
            Instr::new(Opcode::PushConst),
            Instr::with_a(Opcode::JumpIfFalse, 4),
            Instr::new(Opcode::PushConst),
            Instr::new(Opcode::Halt),
        ],
        5,
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

#[test]
fn verify_rejects_bare_instruction_product() {
    let bc = bad_entry(vec![Instr::new(Opcode::Halt)], 0);
    assert_eq!(
        bc.verify().unwrap_err(),
        CompileError::DebugInfoMissing { func: 0, pc: 0 }
    );
}

#[test]
fn verify_rejects_halt_outside_entry_product() {
    // A closure proto ending in halt instead of ret: only funcs[0]
    // may halt.
    let mut f = FuncProto::new(None, 0, 0);
    f.code = vec![Instr::new(Opcode::Halt)];
    f.debug_info = vec![span(0, 1)];
    let mut bc = bad_entry(vec![Instr::new(Opcode::Halt)], 1);
    bc.funcs.push(f);
    assert!(matches!(
        bc.verify().unwrap_err(),
        CompileError::HaltOutsideEntry { pc: 0, .. }
    ));
}
