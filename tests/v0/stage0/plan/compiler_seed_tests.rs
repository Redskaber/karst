//! Cross-crate integration tests for the compiler seed (FP4).
//!
//! Mounted via `tests/runner.rs` (root-tree discipline).
//! The anchor cases run the full source pipeline — reader ->
//! expander -> compiler -> verify — pinning the fib contract and the
//! do->Let->StoreLocal chain; the negative cases exercise cross-layer
//! cap ordering and verify rejections over tampered products.

use karst::compiler::{Bytecode, CompileError, Instr, Opcode, compile, compile_program};
use karst::core::{CoreExpr, LiteralValue};
use karst::expander::{EXPANSION_LIMIT, ExpandCtx, expand, expand_program};
use karst::reader::read_program;
use karst::span::{ByteOffset, FileId, Span};
use karst::syntax::{Phase, ScopeSet, SyntaxObject};

const FILE: FileId = FileId(0);

fn fix_span(start: u32, end: u32) -> Span {
    Span::new(
        FILE,
        ByteOffset(start),
        ByteOffset(end),
        karst::span::ExpansionId::ROOT,
    )
    .expect("fixture span must be well-formed")
}

// ---------- positive: cross-stage consumption ----------

#[test]
fn fib_source_compiles_to_verified_bytecode_end_to_end() {
    // Full chain: source text -> tokens -> stx tree -> CoreExpr ->
    // Bytecode -> verify (the 0.3 + 0.4 + 0.5 three-segment anchor).
    let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))";
    let forms = read_program(FILE, src).expect("fib source must read");
    let items = expand_program(&forms).expect("fib source must expand");
    let bc = compile_program(&items).expect("fib items must compile");
    bc.verify().expect("compiled fib must verify");
    // The recursive shape is present: a named fib closure with a
    // tail-if body and arity-2 calls.
    assert_eq!(bc.funcs.len(), 2);
    let fib = &bc.funcs[1];
    assert_eq!(fib.name.as_ref().map(|s| s.as_str()), Some("fib"));
    assert_eq!(fib.params, 1);
    assert!(fib.code.iter().any(|i| i.op == Opcode::Call && i.a == 2));
    // Tail-if contract: no merge jump inside the closure body (the
    // only jump-if-false targets the else branch right after a ret).
    let ops: Vec<Opcode> = fib.code.iter().map(|i| i.op).collect();
    assert!(!ops.contains(&Opcode::Jump), "tail if has no merge jump");
    assert!(ops.contains(&Opcode::Ret));
    assert_eq!(ops.last(), Some(&Opcode::Ret));
    // debug_info covers every instruction (no bare instructions).
    assert_eq!(fib.debug_info.len(), fib.code.len());
    // spans trace back into the fib define region (single-generation
    // ROOT spans copied from CoreExpr nodes).
    let s = bc.span_of(1, 0).expect("closure pc 0 has a span");
    assert!(
        s.start.0 >= 16 && s.end.0 <= 68,
        "closure pc 0 span must map into the fib body: {s}"
    );
}

#[test]
fn do_desugar_compiles_to_storelocal_chain_end_to_end() {
    // (define (f x) (do (+ x 1) 1 x)) — do desugars to a Let chain at
    // 0.4; the compiler lowers each binding to push + store-local.
    // `+` is a STAGE0_GLOBALS name (the expander would reject `g`).
    let src = "(define (f x) (do (+ x 1) 1 x))";
    let forms = read_program(FILE, src).expect("do source must read");
    let items = expand_program(&forms).expect("do source must expand");
    let bc = compile_program(&items).expect("do items must compile");
    bc.verify().expect("compiled do must verify");
    let f = &bc.funcs[1];
    let ops: Vec<Opcode> = f.code.iter().map(|i| i.op).collect();
    // (+ x 1) discarded binding, 1 discarded, x body: three
    // store-locals after their value forms, body load-local before
    // ret.
    assert_eq!(
        ops,
        vec![
            Opcode::LoadLocal,  // x (arg of +)
            Opcode::PushConst,  // 1
            Opcode::LoadGlobal, // +
            Opcode::Call,
            Opcode::StoreLocal, // _ <- (+ x 1)
            Opcode::PushConst,  // 1
            Opcode::StoreLocal, // _ <- 1
            Opcode::LoadLocal,  // x (body)
            Opcode::Ret,
        ]
    );
    assert_eq!(f.n_locals, 3, "param x + two discard bindings");
}

// ---------- negative ----------

#[test]
fn expander_cap_intercepts_before_compiler_cap() {
    // A synthesized 513-deep tree exceeds the expander budget (512)
    // first — the compiler cap (1024) is unreachable for this input;
    // cross-layer cap ordering is pinned by construction.
    let sp = fix_span(0, 1);
    let mut node = SyntaxObject::int(42, sp, ScopeSet::new(), Phase::Runtime);
    for _ in 0..EXPANSION_LIMIT {
        let head =
            SyntaxObject::symbol("+", sp, ScopeSet::new(), Phase::Runtime).expect("fixture head");
        node = SyntaxObject::list(vec![head, node], sp, ScopeSet::new(), Phase::Runtime);
    }
    let mut ctx = ExpandCtx::new();
    match expand(&mut ctx, &node) {
        Ok(_) => panic!("513-deep tree must be rejected by the expander budget"),
        Err(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("expansion"),
                "expander rejects first, got: {msg}"
            );
        }
    }
}

#[test]
fn source_bounded_tree_compiles_and_hand_built_deeper_is_rejected() {
    // Cap ordering across layers: reader 256 < expander 512 <
    // compiler 1024. A 250-deep source tree (inside every cap)
    // compiles fine; a hand-built 1025-deep CoreExpr chain fails
    // closed with DepthExceeded. (Trees deeper than 256 cannot come
    // from source at all — the reader cap fires first; only macro
    // expansion can produce 257..512, which lands before 0.9.)
    let mut inner = String::from("1");
    for _ in 0..250 {
        inner = format!("(+ 1 {inner})");
    }
    let src = format!("(define (h x) x) (define r {inner})");
    let forms = read_program(FILE, &src).expect("250-deep source is inside the reader cap");
    let items = expand_program(&forms).expect("deep source is within the expander budget");
    let bc = compile_program(&items).expect("source-bounded tree compiles");
    bc.verify().expect("deep-but-legal product verifies");
    // hand-built chain one over the compiler cap
    let mut e = CoreExpr::Literal {
        span: fix_span(0, 1),
        value: LiteralValue::Int(0),
    };
    for _ in 0..1024 {
        e = CoreExpr::Apply {
            span: fix_span(0, 1),
            func: Box::new(CoreExpr::var(fix_span(0, 1), "+").unwrap()),
            args: vec![e],
        };
    }
    assert!(matches!(
        compile(&e).unwrap_err(),
        CompileError::DepthExceeded { depth: 1025, .. }
    ));
}

/// Tamper helper: clone a verified product, mutate one instruction,
/// expect verify to reject.
fn tampered(bc: &Bytecode, edit: impl FnOnce(&mut Instr)) -> Bytecode {
    let mut copy = bc.clone();
    edit(&mut copy.funcs[0].code[0]);
    copy
}

fn verified_product() -> Bytecode {
    let src = "(define (f x) (+ x 1))";
    let forms = read_program(FILE, src).unwrap();
    let items = expand_program(&forms).unwrap();
    let bc = compile_program(&items).unwrap();
    bc.verify().unwrap();
    bc
}

#[test]
fn tampered_jump_target_is_rejected_by_verify() {
    let bc = verified_product();
    // entry pc 0 becomes a jump to pc 9 (beyond the code length).
    let bad = tampered(&bc, |i| {
        i.op = Opcode::Jump;
        i.a = 9;
    });
    assert!(matches!(
        bad.verify().unwrap_err(),
        CompileError::JumpTargetOutOfRange {
            pc: 0,
            target: 9,
            ..
        }
    ));
}

#[test]
fn tampered_debug_info_is_rejected_by_verify() {
    let mut bc = verified_product();
    bc.funcs[0].debug_info.truncate(1);
    assert!(matches!(
        bc.verify().unwrap_err(),
        CompileError::DebugInfoMissing { func: 0, .. }
    ));
}

#[test]
fn tampered_const_index_is_rejected_by_verify() {
    let bc = verified_product();
    let mut bad = bc.clone();
    bad.funcs[0].code[0] = Instr::with_a(Opcode::PushConst, 9);
    assert!(matches!(
        bad.verify().unwrap_err(),
        CompileError::ConstIndexOutOfRange {
            pc: 0,
            index: 9,
            ..
        }
    ));
}

#[test]
fn tampered_local_slot_is_rejected_by_verify() {
    let bc = verified_product();
    let mut bad = bc.clone();
    bad.funcs[0].code[0] = Instr::with_a(Opcode::LoadLocal, 7);
    assert!(matches!(
        bad.verify().unwrap_err(),
        CompileError::LocalSlotOutOfRange { pc: 0, slot: 7, .. }
    ));
}
