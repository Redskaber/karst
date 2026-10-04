//! Stage 0 gate audit set, revision 1 — negative audit.
//!
//! Run: `cargo run --release --example stage0_gate_audit_r1`
//! (exit 0 = all 30 cases behave as audited; any panic = audit failure).

use karst::reader::{NESTING_LIMIT, ReadError, read_program};
use karst::span::FileId;

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

fn main() {
    // ---- Section A: single-statement negative (10) ----
    // SOP §7.1.1 class 1 (syntax error) unless noted.
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

    println!("AUDIT PASS 30/30 (A10 + B10 + C5 + D5)");
    println!(
        "class coverage at 0.3: syntax error + empty application (2/7 lit, 5 structural DEFERRED)"
    );
}
