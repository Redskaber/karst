//! karst-reader crate-level integration tests: verify the public API

use karst_reader::{Delimiter, NESTING_LIMIT, ReadError, TokenKind, lex, read_program};
use karst_span::{ByteOffset, FileId};

fn file() -> FileId {
    FileId(0)
}

#[test]
fn public_api_exports_documented_surface() {
    // Compile-level surface check: every documented name is reachable
    // from the crate root (SOP §10.1 rule 4 explicit re-export list).
    let tokens = lex(file(), "(x)").expect("lex via public path");
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].kind_name(), "delimiter");
    assert_eq!(tokens[0].kind, TokenKind::Delimiter(Delimiter::Open));
    assert_eq!(tokens[3].kind, TokenKind::Eof);

    let forms = read_program(file(), "(x 1)").expect("read via public path");
    assert_eq!(forms.len(), 1);
    assert_eq!(forms[0].datum.to_string(), "(x 1)");
    assert_eq!(NESTING_LIMIT, 256, "Stage 0 nesting cap is 256");
    let _err: ReadError = ReadError::EmptyApplication { at: ByteOffset(0) };
}

#[test]
fn public_reader_builds_nested_program() {
    // A representative nested program: every node comes back with an
    // exact span and the reader stamps (Runtime phase, empty scopes).
    let forms = read_program(file(), "(define (f x) (add x 1))").expect("program reads");
    assert_eq!(forms.len(), 1);
    let root = &forms[0];
    assert_eq!(root.datum.to_string(), "(define (f x) (add x 1))");
    assert_eq!((root.span.start.0, root.span.end.0), (0, 24));
    let mut count = 0usize;
    root.visit_spans(&mut |_s| count += 1);
    assert_eq!(
        count, 9,
        "root + define + f-form + f + x + add-form + add + x + 1"
    );
}

#[test]
fn public_path_rejects_empty_application() {
    let err = read_program(file(), "()").expect_err("empty application rejected via public path");
    assert_eq!(err, ReadError::EmptyApplication { at: ByteOffset(0) });
    assert!(err.to_string().contains("no operator"));
}

#[test]
fn public_path_rejects_unclosed_list() {
    let err = read_program(file(), "(f 1").expect_err("unclosed list rejected via public path");
    assert_eq!(
        err,
        ReadError::UnclosedList {
            opened_at: ByteOffset(0)
        }
    );
}

#[test]
fn public_path_rejects_malformed_number() {
    let err = read_program(file(), "(f 1.2.3)").expect_err("malformed number rejected");
    assert_eq!(
        err,
        ReadError::MalformedNumber {
            text: "1.2.3".to_owned()
        }
    );
}

#[test]
fn public_path_rejects_invalid_bool() {
    let err = read_program(file(), "#true").expect_err("invalid bool rejected via public path");
    assert_eq!(
        err,
        ReadError::InvalidBool {
            text: "#true".to_owned()
        }
    );
}

#[test]
fn public_path_rejects_unterminated_string() {
    let err = read_program(file(), "\"oops").expect_err("unterminated string rejected");
    assert_eq!(
        err,
        ReadError::UnterminatedString {
            started_at: ByteOffset(0)
        }
    );
}

#[test]
fn public_path_rejects_nesting_over_limit() {
    let mut src = String::new();
    for _ in 0..(NESTING_LIMIT + 1) {
        src.push('(');
    }
    src.push('x');
    for _ in 0..(NESTING_LIMIT + 1) {
        src.push(')');
    }
    let err = read_program(file(), &src).expect_err("over-cap nesting rejected via public path");
    assert_eq!(
        err,
        ReadError::NestingDepthExceeded {
            depth: NESTING_LIMIT + 1,
            limit: NESTING_LIMIT,
        }
    );
}
