//! Stage 0.3 plan tests — karst-reader seed MUV, cross-crate integration level.

// Shared helpers: mounted once by the runner entry (tests/runner.rs) and
// imported here via `super::`.
use super::common::{covering_span, open_token};
use karst::WORKSPACE_VERSION;
use karst::reader::{NESTING_LIMIT, ReadError, lex, read_program};
use karst::span::{ByteOffset, FileId};
use karst::syntax::SyntaxError;

fn file() -> FileId {
    FileId(0)
}

#[test]
fn reader_pipeline_token_span_and_tree_reduction() {
    // Positive + cross-stage consumption: the full read pipeline over a
    // small program — the lexed token stream reduces to the same
    // covering span the parser stamps on the list node (the real-token
    // reduction pattern, formerly proven through the retired TokenLike
    // stand-in), and the tree folds via `join` (0.4 expander contract).
    let tokens = lex(file(), "(add 1 2)").expect("program lexes via aggregate path");
    let covering = covering_span(&tokens[..6]).expect("token spans must reduce");
    assert_eq!((covering.start.0, covering.end.0), (0, 9));

    let forms = read_program(file(), "(add 1 2)").expect("program reads via aggregate path");
    assert_eq!(forms.len(), 1);
    let root = &forms[0];
    assert_eq!(root.datum.to_string(), "(add 1 2)");
    assert_eq!((root.span.start.0, root.span.end.0), (0, 9));

    let mut spans = Vec::new();
    root.visit_spans(&mut |s| spans.push(s));
    assert_eq!(spans.len(), 4, "list + add + 1 + 2");
    let folded = spans[1..]
        .iter()
        .try_fold(spans[0], |acc, s| acc.join(s))
        .expect("same-file same-generation node spans must reduce");
    assert_eq!((folded.start.0, folded.end.0), (0, 9));

    // The real-token reduction helper and the reader's own list span
    // agree — the retired stand-in's contract now holds against the
    // production token type.
    let hand_built = [
        open_token(super::common::root_span(0, 0, 1)),
        super::common::ident_token("add", super::common::root_span(0, 1, 4)),
        super::common::int_token(1, super::common::root_span(0, 5, 6)),
        super::common::int_token(2, super::common::root_span(0, 7, 8)),
        super::common::close_token(super::common::root_span(0, 8, 9)),
    ];
    let hand_covering = covering_span(&hand_built).expect("hand-built tokens must reduce");
    assert_eq!(hand_covering, root.span);

    assert_eq!(WORKSPACE_VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn multi_form_cjk_program_carries_byte_precise_spans() {
    // Positive + cross-stage consumption: CJK identifiers are
    // multi-byte — spans must be byte-precise (docs/stage0.md §12.1
    // byte-offset representation). CJK here is the *tested object*
    // (SOP §10.5 registered fixture whitelist).
    let src = "(计数 1) (数 2)";
    let forms = read_program(file(), src).expect("CJK program reads");
    assert_eq!(forms.len(), 2);
    // `(`=1B, the CJK identifier=one 6-byte atom (1..7), ` `, `1`=1B,
    // `)` → first list 0..10.
    assert_eq!((forms[0].span.start.0, forms[0].span.end.0), (0, 10));
    // Second list starts after the space at 10.
    assert_eq!((forms[1].span.start.0, forms[1].span.end.0), (11, 18));
    assert_eq!(forms[0].datum.to_string(), "(计数 1)");
    let mut order = Vec::new();
    forms[0].visit_spans(&mut |s| order.push((s.start.0, s.end.0)));
    assert_eq!(order, vec![(0, 10), (1, 7), (8, 9)]);
}

#[test]
fn aggregate_path_rejects_unclosed_list() {
    // Negative (SOP §7.1.1 error class 1 — syntax error): unclosed
    // delimiter via the aggregate path, byte-precise location. The
    // inner (x) closes; the outer list (byte 0) is the unclosed one —
    // the SOP §7.1.1 example verbatim.
    let err = read_program(file(), "(lambda (x)")
        .expect_err("unclosed delimiter must be rejected via aggregate path");
    assert_eq!(
        err,
        ReadError::UnclosedList {
            opened_at: ByteOffset(0)
        }
    );
}

#[test]
fn aggregate_path_rejects_empty_application() {
    // Negative (SOP §7.1.1 error class 3 — empty syntax object): `()` has no
    // operator; the reader rejects it fail-closed.
    let err = read_program(file(), "()").expect_err("empty application must be rejected");
    assert_eq!(err, ReadError::EmptyApplication { at: ByteOffset(0) });
    assert!(err.to_string().contains("no operator"));
}

#[test]
fn aggregate_path_enforces_nesting_limit() {
    // Negative: the 256-level cap is enforced through the aggregate
    // path, and the exported constant pins the cap value.
    assert_eq!(NESTING_LIMIT, 256);
    let mut src = String::new();
    for _ in 0..=NESTING_LIMIT {
        src.push('(');
    }
    src.push('x');
    for _ in 0..=NESTING_LIMIT {
        src.push(')');
    }
    let err = read_program(file(), &src).expect_err("over-cap nesting must be rejected");
    assert_eq!(
        err,
        ReadError::NestingDepthExceeded {
            depth: NESTING_LIMIT + 1,
            limit: NESTING_LIMIT,
        }
    );
}

#[test]
fn error_recovery_is_call_isolated() {
    // Negative (error recovery semantics, Stage 0): a failing source
    // returns a structured error (never a panic), and an independent
    // read of the corrected source succeeds — call-level isolation.
    let broken = "(f 1";
    let fixed = "(f 1)";
    let err = read_program(file(), broken).expect_err("broken source must fail");
    assert!(matches!(err, ReadError::UnclosedList { .. }));
    let forms = read_program(file(), fixed).expect("fixed source must read after the failure");
    assert_eq!(forms.len(), 1);
    // A second failing read after the success stays a clean error.
    assert!(read_program(file(), broken).is_err());
}

#[test]
fn invalid_symbol_message_is_byte_identical_to_syntax_layer() {
    // Negative (single-source dictionary, SOP §10.5 / R8): the same
    // source concept renders the same English text in the reader and
    // the syntax layer — the parity-guard prerequisite.
    let reader_err = read_program(file(), "a\u{0}b").expect_err("control char must fail");
    let ReadError::InvalidSymbol { text } = &reader_err else {
        panic!("must be the InvalidSymbol variant, got {reader_err}");
    };
    let syntax_err = SyntaxError::InvalidSymbol { text: text.clone() };
    assert_eq!(reader_err.to_string(), syntax_err.to_string());
}

#[test]
fn aggregate_path_rejects_malformed_number_mid_program() {
    // Negative: a malformed number between two valid forms fails the
    // whole read with the offending atom as the payload — the error
    // names the exact broken token.
    let err = read_program(file(), "(f 1) 2.3.4 (g)")
        .expect_err("malformed number mid-program must be rejected");
    assert_eq!(
        err,
        ReadError::MalformedNumber {
            text: "2.3.4".to_owned()
        }
    );
}

#[test]
fn aggregate_path_rejects_stray_close_at_top_level() {
    // Negative: a `)` with no matching open is rejected with its exact
    // byte offset, even when valid forms precede it.
    let err = read_program(file(), "(a 1) ) (b 2)")
        .expect_err("stray close after valid forms must be rejected");
    assert_eq!(err, ReadError::UnexpectedClose { at: ByteOffset(6) });
}
