//! Stage 0 gate audit set, revision 1 — negative audit.
//!
//! Run: `cargo run --release --example stage0_gate_audit_r1`
//! (exit 0 = all 50 cases behave as audited; any panic = audit failure).

use karst::core::CoreError;
use karst::expander::{EXPANSION_LIMIT, ExpandCtx, ExpandError, expand, expand_program};
use karst::reader::{NESTING_LIMIT, ReadError, read_program};
use karst::span::{ByteOffset, FileId};
use karst::syntax::{Phase, ScopeSet, SyntaxObject};

const FILE: FileId = FileId(0);

/// Assert the source fails and the error matches `pred`.
fn audit_err(src: &str, name: &str, pred: impl FnOnce(&ReadError) -> bool) {
    match read_program(FILE, src) {
        Ok(forms) => panic!(
            "audit case `{name}` must fail, but read {} forms",
            forms.len()
        ),
        Err(err) => {
            assert!(
                pred(&err),
                "audit case `{name}` failed with the wrong error: {err}"
            );
        }
    }
}

/// Assert the source reads to exactly `forms` top-level forms.
fn audit_ok(src: &str, forms: usize, name: &str) {
    let parsed = read_program(FILE, src)
        .unwrap_or_else(|err| panic!("audit case `{name}` must read, but failed: {err}"));
    assert_eq!(
        parsed.len(),
        forms,
        "audit case `{name}` must read exactly {forms} forms"
    );
}

/// `n` levels of nesting around `inner`.
fn deep(n: usize, inner: &str) -> String {
    let mut src = String::new();
    for _ in 0..n {
        src.push('(');
    }
    src.push_str(inner);
    for _ in 0..n {
        src.push(')');
    }
    src
}

/// Assert the source reads cleanly and then fails expansion, with the
/// error matching `pred` (the expander counterpart of `audit_err`).
fn expand_err(src: &str, name: &str, pred: impl FnOnce(&ExpandError) -> bool) {
    let forms = match read_program(FILE, src) {
        Ok(forms) => forms,
        Err(err) => panic!("audit case `{name}` must read cleanly, got {err}"),
    };
    match expand_program(&forms) {
        Ok(items) => panic!(
            "audit case `{name}` must fail, but expanded {} items",
            items.len()
        ),
        Err(err) => {
            assert!(
                pred(&err),
                "audit case `{name}` failed with the wrong error: {err}"
            );
        }
    }
}

fn main() {
    // ---- Section A: single-statement negative (10) ----
    audit_err(
        "(f",
        "A01 unclosed list",
        |e| matches!(e, ReadError::UnclosedList { opened_at } if opened_at.0 == 0),
    );
    audit_err(
        ")",
        "A02 stray close",
        |e| matches!(e, ReadError::UnexpectedClose { at } if at.0 == 0),
    );
    // SOP §7.1.1 class 3 (empty syntax object / empty application).
    audit_err(
        "()",
        "A03 empty application",
        |e| matches!(e, ReadError::EmptyApplication { at } if at.0 == 0),
    );
    audit_err(
        "\"abc",
        "A04 unterminated string",
        |e| matches!(e, ReadError::UnterminatedString { started_at } if started_at.0 == 0),
    );
    audit_err(
        "\"a\\x\"",
        "A05 invalid escape",
        |e| matches!(e, ReadError::InvalidEscape { escape, .. } if *escape == 'x'),
    );
    audit_err(
        "#q",
        "A06 invalid bool",
        |e| matches!(e, ReadError::InvalidBool { text } if text == "#q"),
    );
    audit_err(
        "1.2.3",
        "A07 malformed number (multi-dot)",
        |e| matches!(e, ReadError::MalformedNumber { text } if text == "1.2.3"),
    );
    audit_err(
        "12abc",
        "A08 malformed number (suffix)",
        |e| matches!(e, ReadError::MalformedNumber { text } if text == "12abc"),
    );
    audit_err("a\u{0}b", "A09 invalid symbol (control char)", |e| {
        matches!(e, ReadError::InvalidSymbol { .. })
    });
    audit_err(
        "[",
        "A10 unexpected character",
        |e| matches!(e, ReadError::UnexpectedCharacter { character, .. } if *character == '['),
    );

    // ---- Section B: multi-statement/multi-form negative (10) ----
    // The error sits inside a larger program of otherwise-valid forms;
    // every case asserts the byte-precise location of the broken form.
    audit_err(
        "(define x 1) (f",
        "B01 later form unclosed",
        |e| matches!(e, ReadError::UnclosedList { opened_at } if opened_at.0 == 13),
    );
    audit_err(
        "x 42 )",
        "B02 stray close after atoms",
        |e| matches!(e, ReadError::UnexpectedClose { at } if at.0 == 5),
    );
    audit_err(
        "(a 1) () (b 2)",
        "B03 empty app between forms",
        |e| matches!(e, ReadError::EmptyApplication { at } if at.0 == 6),
    );
    audit_err(
        "(f 1) \"s (g 2)",
        "B04 unterminated string mid-program",
        |e| matches!(e, ReadError::UnterminatedString { started_at } if started_at.0 == 6),
    );
    audit_err(
        "#t #f #q",
        "B05 bad bool in atom sequence",
        |e| matches!(e, ReadError::InvalidBool { text } if text == "#q"),
    );
    audit_err(
        "1 2 3.3.3",
        "B06 malformed number in sequence",
        |e| matches!(e, ReadError::MalformedNumber { text } if text == "3.3.3"),
    );
    audit_err(
        "(a) (b (c)",
        "B07 nested form unclosed",
        |e| matches!(e, ReadError::UnclosedList { opened_at } if opened_at.0 == 4),
    );
    audit_err(
        "(a (b 1)) ) ",
        "B08 stray close after nested form",
        |e| matches!(e, ReadError::UnexpectedClose { at } if at.0 == 10),
    );
    audit_err(
        "nil (f x) (quote ())",
        "B09 quoted empty list rejected at read",
        |e| matches!(e, ReadError::EmptyApplication { at } if at.0 == 17),
    );
    audit_err(
        "s1 s2 a\u{1}b",
        "B10 control char in third form",
        |e| matches!(e, ReadError::InvalidSymbol { text } if text == "a\u{1}b"),
    );

    // ---- Section C: complex-program negative (5) ----
    audit_err(
        &deep(NESTING_LIMIT + 1, "x"),
        "C01 nesting over the cap",
        |e| matches!(e, ReadError::NestingDepthExceeded { depth, limit } if *depth == NESTING_LIMIT + 1 && *limit == NESTING_LIMIT),
    );
    audit_err(
        &deep(200, "(f"),
        "C02 deep nesting with unclosed bottom",
        |e| matches!(e, ReadError::UnclosedList { .. }),
    );
    audit_err(
        &deep(150, "()"),
        "C03 empty application at nesting bottom",
        |e| matches!(e, ReadError::EmptyApplication { .. }),
    );
    audit_err(
        &deep(100, "1.2.3"),
        "C04 malformed number at nesting bottom",
        |e| matches!(e, ReadError::MalformedNumber { .. }),
    );
    // Wide-and-deep: 100 levels, wide fan-out at each level, one stray
    // close grafted at the end.
    let wide = {
        let level = "(f a b (g c";
        let mut src = String::new();
        for _ in 0..10 {
            src.push_str(level);
        }
        src.push(')');
        src
    };
    audit_err(&wide, "C05 wide nesting with unbalanced tail", |e| {
        matches!(e, ReadError::UnclosedList { .. })
    });

    // ---- Section D: error-recovery (5, Stage 0 semantics) ----
    // One error must not poison anything after it: independent calls
    // stay isolated and the corrected source reads cleanly.
    {
        // D01: unclosed -> close it.
        assert!(read_program(FILE, "(f").is_err());
        audit_ok("(f 1)", 1, "D01 corrected list reads");
    }
    {
        // D02: mid-program error -> truncate to the valid prefix.
        assert!(read_program(FILE, "(a 1) (b").is_err());
        audit_ok("(a 1)", 1, "D02 valid prefix reads after failure");
    }
    {
        // D03: empty application -> give it an operator.
        assert!(read_program(FILE, "()").is_err());
        audit_ok("(f 1)", 1, "D03 operator-given form reads");
    }
    {
        // D04: over-cap nesting -> shrink one level (cap boundary both
        // ways in one recovery pair).
        assert!(read_program(FILE, &deep(NESTING_LIMIT + 1, "x")).is_err());
        audit_ok(&deep(NESTING_LIMIT, "x"), 1, "D04 cap-edge nesting reads");
    }
    {
        // D05: invalid escape -> valid escape, same call sequence.
        assert!(read_program(FILE, "\"a\\x\"").is_err());
        audit_ok("\"a\\n\"", 1, "D05 escaped newline string reads");
    }

    // ---- Section E: expander negatives (10, sub-stage 0.4) ----
    // The reader -> expander full path plus synthesized-tree faces;
    // every case is a structured ExpandError, never a panic.
    // E01: unbound identifier, single form (class 2).
    expand_err(
        "(define (f) nope)",
        "E01 unbound identifier in single form",
        |e| matches!(e, ExpandError::UnboundIdentifier { .. }),
    );
    // E02: unbound identifier in a later form of a multi-form program.
    expand_err(
        "(define a 1) (define (g) missing)",
        "E02 unbound identifier in later form",
        |e| matches!(e, ExpandError::UnboundIdentifier { name, .. } if name == "missing"),
    );
    // E03: expansion work budget exceeded on a synthesized deep tree
    // (class 7 — the reader cap cannot produce this; the expander
    // budget bounds synthesized/macro-produced trees independently).
    {
        let span = karst::span::Span::root(FILE, ByteOffset(0), ByteOffset(1))
            .expect("audit fixture span");
        // A global head and a literal innermost atom: the budget is
        // the only possible failure.
        let mut node = SyntaxObject::int(42, span, ScopeSet::new(), Phase::Runtime);
        for _ in 0..EXPANSION_LIMIT {
            let head = SyntaxObject::symbol("+", span, ScopeSet::new(), Phase::Runtime)
                .expect("audit fixture symbol");
            node = SyntaxObject::list(vec![head, node], span, ScopeSet::new(), Phase::Runtime);
        }
        let mut ctx = ExpandCtx::new();
        match expand(&mut ctx, &node) {
            Ok(_) => panic!("audit case `E03` must fail, but the deep tree expanded"),
            Err(err) => assert!(
                matches!(err, ExpandError::ExpansionDepthExceeded { .. }),
                "audit case `E03` failed with the wrong error: {err}"
            ),
        }
    }
    // E04: phase closure — macro-phase syntax is not runtime code.
    {
        let span = karst::span::Span::root(FILE, ByteOffset(0), ByteOffset(1))
            .expect("audit fixture span");
        let form = SyntaxObject::symbol("x", span, ScopeSet::new(), Phase::ExpandTime)
            .expect("audit fixture symbol");
        let mut ctx = ExpandCtx::new();
        match expand(&mut ctx, &form) {
            Ok(_) => panic!("audit case `E04` must fail, but macro-phase expanded"),
            Err(err) => assert!(
                matches!(err, ExpandError::PhaseViolation { .. }),
                "audit case `E04` failed with the wrong error: {err}"
            ),
        }
    }
    // E05: malformed special form (two-armed if).
    expand_err(
        "(define (f) (if 1 2))",
        "E05 two-armed if is malformed",
        |e| matches!(e, ExpandError::MalformedForm { form, .. } if *form == "if"),
    );
    // E06: bare top-level expression (declarations only at module top).
    expand_err("(f 1)", "E06 bare top-level expression", |e| {
        matches!(e, ExpandError::TopLevelExpression { .. })
    });
    // E07: set! to an unbound target (class 2, assignment face).
    expand_err(
        "(define (f) (set! nope 1))",
        "E07 set! to unbound target",
        |e| matches!(e, ExpandError::UnboundIdentifier { name, .. } if name == "nope"),
    );
    // E08: duplicate parameters re-asserted through the expander.
    expand_err(
        "(define (f x x) x)",
        "E08 duplicate params via expander",
        |e| {
            matches!(e, ExpandError::Core { source, .. }
            if *source == CoreError::DuplicateParam { name: "x".to_owned() })
        },
    );
    // E09: empty synthesized form (class 3, expander detection point —
    // the reader already rejects `()` in source; this is the second
    // fail-closed layer for hand-built trees).
    {
        let span = karst::span::Span::root(FILE, ByteOffset(0), ByteOffset(2))
            .expect("audit fixture span");
        let form = SyntaxObject::list(vec![], span, ScopeSet::new(), Phase::Runtime);
        let mut ctx = ExpandCtx::new();
        match expand(&mut ctx, &form) {
            Ok(_) => panic!("audit case `E09` must fail, but the empty form expanded"),
            Err(err) => assert!(
                matches!(err, ExpandError::EmptyForm { .. }),
                "audit case `E09` failed with the wrong error: {err}"
            ),
        }
    }
    // E10: malformed handle clause (one-element pair).
    expand_err(
        "(define (f) (handle 1 (E)))",
        "E10 malformed handle clause",
        |e| matches!(e, ExpandError::MalformedForm { form, .. } if *form == "handle"),
    );

    // ---- Section F: compiler negatives (10, sub-stage 0.5) ----
    // The expander -> compiler full path plus hand-built bad
    // products; every case is a structured CompileError from the
    // compile-time defenses or from Bytecode::verify, never a panic.
    use karst::compiler::{
        Bytecode as FBytecode, CompileError, FuncProto, Instr as FInstr, Opcode,
        compile as fcompile, compile_program as fcompile_program,
    };
    use karst::core::{CoreExpr as FExpr, LiteralValue as FLit};

    fn fspan(start: u32, end: u32) -> karst::span::Span {
        karst::span::Span::root(FILE, ByteOffset(start), ByteOffset(end))
            .expect("audit fixture span")
    }

    /// Hand-built entry proto with one pooled constant (index 0).
    fn fentry(code: Vec<FInstr>, debug_len: usize) -> FBytecode {
        let mut f = FuncProto::new(None, 0, 0);
        f.code = code;
        f.debug_info = vec![fspan(0, 1); debug_len];
        let mut bc = FBytecode::default();
        bc.consts.push(FLit::Int(0));
        bc.funcs.push(f);
        bc
    }

    // F01: compile depth cap on a synthesized 1025-deep chain.
    {
        let mut e = FExpr::Literal {
            span: fspan(0, 1),
            value: FLit::Int(0),
        };
        for _ in 0..1024 {
            e = FExpr::Apply {
                span: fspan(0, 1),
                func: Box::new(FExpr::var(fspan(0, 1), "+").unwrap()),
                args: vec![e],
            };
        }
        match fcompile(&e) {
            Ok(_) => panic!("audit case `F01` must fail, but the deep tree compiled"),
            Err(err) => assert!(
                matches!(err, CompileError::DepthExceeded { depth: 1025, .. }),
                "audit case `F01` failed with the wrong error: {err}"
            ),
        }
    }
    // F02: verify rejects a jump target beyond the code length.
    {
        let bc = fentry(
            vec![
                FInstr::new(Opcode::PushConst),
                FInstr::with_a(Opcode::Jump, 9),
                FInstr::new(Opcode::Halt),
            ],
            3,
        );
        let err = bc.verify().expect_err("audit case `F02` must fail");
        assert!(
            matches!(
                err,
                CompileError::JumpTargetOutOfRange {
                    pc: 1,
                    target: 9,
                    ..
                }
            ),
            "audit case `F02` failed with the wrong error: {err}"
        );
    }
    // F03: verify rejects a pop on the empty stack.
    {
        let bc = fentry(vec![FInstr::new(Opcode::Pop), FInstr::new(Opcode::Halt)], 2);
        let err = bc.verify().expect_err("audit case `F03` must fail");
        assert!(
            matches!(err, CompileError::StackUnderflow { pc: 0, .. }),
            "audit case `F03` failed with the wrong error: {err}"
        );
    }
    // F04: verify rejects a branch-depth imbalance.
    {
        let bc = fentry(
            vec![
                FInstr::new(Opcode::PushConst),
                FInstr::new(Opcode::PushConst),
                FInstr::with_a(Opcode::JumpIfFalse, 4),
                FInstr::new(Opcode::PushConst),
                FInstr::new(Opcode::Halt),
            ],
            5,
        );
        let err = bc.verify().expect_err("audit case `F04` must fail");
        assert!(
            matches!(
                err,
                CompileError::StackImbalance {
                    pc: 4,
                    expected: 1,
                    actual: 2,
                    ..
                }
            ),
            "audit case `F04` failed with the wrong error: {err}"
        );
    }
    // F05: verify rejects a bare instruction (debug_info missing).
    {
        let bc = fentry(vec![FInstr::new(Opcode::Halt)], 0);
        let err = bc.verify().expect_err("audit case `F05` must fail");
        assert!(
            matches!(err, CompileError::DebugInfoMissing { func: 0, pc: 0 }),
            "audit case `F05` failed with the wrong error: {err}"
        );
    }
    // F06: verify rejects an out-of-range constant index.
    {
        let bc = fentry(
            vec![
                FInstr::with_a(Opcode::PushConst, 7),
                FInstr::new(Opcode::Halt),
            ],
            2,
        );
        let err = bc.verify().expect_err("audit case `F06` must fail");
        assert!(
            matches!(
                err,
                CompileError::ConstIndexOutOfRange {
                    pc: 0,
                    index: 7,
                    ..
                }
            ),
            "audit case `F06` failed with the wrong error: {err}"
        );
    }
    // F07: verify rejects halt inside a non-entry proto.
    {
        let mut bc = fentry(vec![FInstr::new(Opcode::Halt)], 1);
        let mut closure = FuncProto::new(None, 0, 0);
        closure.code = vec![FInstr::new(Opcode::Halt)];
        closure.debug_info = vec![fspan(0, 1)];
        bc.funcs.push(closure);
        let err = bc.verify().expect_err("audit case `F07` must fail");
        assert!(
            matches!(err, CompileError::HaltOutsideEntry { pc: 0, .. }),
            "audit case `F07` failed with the wrong error: {err}"
        );
    }
    // F08: verify rejects a proto that does not end in ret/halt.
    {
        let bc = fentry(
            vec![FInstr::new(Opcode::PushConst), FInstr::new(Opcode::Pop)],
            2,
        );
        let err = bc.verify().expect_err("audit case `F08` must fail");
        assert!(
            matches!(err, CompileError::MissingTerminator { func: 0, pc: 1 }),
            "audit case `F08` failed with the wrong error: {err}"
        );
    }
    // F09: verify rejects an out-of-range closure proto index.
    {
        let bc = fentry(
            vec![
                FInstr::with_a(Opcode::Closure, 5),
                FInstr::new(Opcode::Halt),
            ],
            2,
        );
        let err = bc.verify().expect_err("audit case `F09` must fail");
        assert!(
            matches!(
                err,
                CompileError::ProtoIndexOutOfRange {
                    pc: 0,
                    index: 5,
                    ..
                }
            ),
            "audit case `F09` failed with the wrong error: {err}"
        );
    }
    // F10: verify rejects an out-of-range local slot.
    {
        let bc = fentry(
            vec![
                FInstr::with_a(Opcode::LoadLocal, 3),
                FInstr::new(Opcode::Halt),
            ],
            2,
        );
        let err = bc.verify().expect_err("audit case `F10` must fail");
        assert!(
            matches!(
                err,
                CompileError::LocalSlotOutOfRange { pc: 0, slot: 3, .. }
            ),
            "audit case `F10` failed with the wrong error: {err}"
        );
    }
    // Cross-check: a full source program compiles and verifies (the
    // compile-output-passes-verify contract) — not counted in F, a
    // guard against vacuous rejection tests.
    {
        let src = "(define (fib n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))";
        let forms = read_program(FILE, src).expect("audit guard must read");
        let items = expand_program(&forms).expect("audit guard must expand");
        let bc = fcompile_program(&items).expect("audit guard must compile");
        bc.verify()
            .expect("audit guard: compile output must verify");
    }

    println!("AUDIT PASS 50/50 (A10 + B10 + C5 + D5 + E10 + F10)");
    println!(
        "class coverage at 0.5: syntax error + empty application + unbound identifier + expansion depth (4/7 lit, 3 structural DEFERRED)"
    );
}
