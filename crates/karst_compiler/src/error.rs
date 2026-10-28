//! @path: karst/crates/karst_compiler/error.rs
//! @author: redskaber
//! @datetime: 2026-10-13
//! @discription: karst::crates::karst_compiler::error
//!
//! bytecode generation from CoreExpr - the structured failure surface of bytecode generation

use core::fmt;

use karst_span::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    StackUnderflow {
        pc: usize,
        span: Span,
    },
    StackImbalance {
        pc: usize,
        expected: u32,
        actual: u32,
        span: Span,
    },
    DepthExceeded {
        depth: usize,
        span: Span,
    },
    UnpatchedJump {
        at: usize,
        span: Span,
    },
    // verify
    EmptyFuncTable,
    DebugInfoMissing {
        func: usize,
        pc: usize,
    },
    MissingTerminator {
        func: usize,
        pc: usize,
    },
    ConstIndexOutOfRange {
        pc: usize,
        index: usize,
        span: Span,
    },
    GlobalIndexOutOfRange {
        pc: usize,
        index: usize,
        span: Span,
    },
    LocalSlotOutOfRange {
        pc: usize,
        slot: usize,
        span: Span,
    },
    ProtoIndexOutOfRange {
        pc: usize,
        index: usize,
        span: Span,
    },
    JumpTargetOutOfRange {
        pc: usize,
        target: usize,
        span: Span,
    },
    HaltOutsideEntry {
        pc: usize,
        span: Span,
    },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::DepthExceeded { depth, span } => write!(
                f,
                "compile depth `{depth}` exceeds limit `{}` at {span}",
                crate::COMPILE_DEPTH_LIMIT
            ),
            CompileError::UnpatchedJump { at, span } => {
                write!(f, "unpatched jump placeholder at `{at}` ({span})")
            }
            CompileError::StackUnderflow { pc, span } => {
                write!(f, "stack underflow at pc `{pc}` ({span})")
            }
            CompileError::StackImbalance {
                pc,
                expected,
                actual,
                span,
            } => write!(
                f,
                "stack depth `{actual}` at pc `{pc}` disagrees with predecessor depth `{expected}` ({span})"
            ),
            CompileError::ConstIndexOutOfRange { pc, index, span } => write!(
                f,
                "constant index `{index}` out of range at pc `{pc}` ({span})"
            ),
            CompileError::GlobalIndexOutOfRange { pc, index, span } => write!(
                f,
                "global index `{index}` out of range at pc `{pc}` ({span})"
            ),
            CompileError::LocalSlotOutOfRange { pc, slot, span } => {
                write!(f, "local slot `{slot}` out of range at pc `{pc}` ({span})")
            }
            CompileError::ProtoIndexOutOfRange { pc, index, span } => write!(
                f,
                "proto index `{index}` out of range at pc `{pc}` ({span})"
            ),
            CompileError::JumpTargetOutOfRange { pc, target, span } => write!(
                f,
                "jump target `{target}` out of range at pc `{pc}` ({span})"
            ),
            CompileError::DebugInfoMissing { func, pc } => write!(
                f,
                "debug info missing for func `{func}` pc `{pc}` (bare instruction)"
            ),
            CompileError::MissingTerminator { func, pc } => write!(
                f,
                "func `{func}` must end with `ret` or `halt` (last pc `{pc}`)"
            ),
            CompileError::HaltOutsideEntry { pc, span } => {
                write!(f, "`halt` outside entry proto at pc `{pc}` ({span})")
            }
            CompileError::EmptyFuncTable => {
                write!(f, "bytecode has no function proto (entry proto missing)")
            }
        }
    }
}
impl std::error::Error for CompileError {}

#[cfg(test)]
mod tests {
    //! CompileError surface tests (FP1): construction, span/pc payloads,
    //! R8 message shapes (lowercase start, no trailing period, backtick
    //! payloads, technical anchors kept verbatim).

    use super::*;
    use karst_span::{ByteOffset, ExpansionId, FileId};

    fn span(start: u32, end: u32) -> Span {
        Span::new(
            FileId(0),
            ByteOffset(start),
            ByteOffset(end),
            ExpansionId::ROOT,
        )
        .expect("test fixture span must be well-formed")
    }

    #[test]
    fn depth_exceeded_display_keeps_r8_shape() {
        let err = CompileError::DepthExceeded {
            depth: 1025,
            span: span(3, 7),
        };
        assert_eq!(
            err.to_string(),
            "compile depth `1025` exceeds limit `1024` at file#0:3..7"
        );
    }

    #[test]
    fn unpatched_jump_and_underflow_display_forms() {
        let unpatched = CompileError::UnpatchedJump {
            at: 4,
            span: span(0, 2),
        };
        assert_eq!(
            unpatched.to_string(),
            "unpatched jump placeholder at `4` (file#0:0..2)"
        );
        let underflow = CompileError::StackUnderflow {
            pc: 9,
            span: span(1, 3),
        };
        assert_eq!(
            underflow.to_string(),
            "stack underflow at pc `9` (file#0:1..3)"
        );
    }

    #[test]
    fn stack_imbalance_names_both_depths() {
        let err = CompileError::StackImbalance {
            pc: 6,
            expected: 1,
            actual: 2,
            span: span(10, 12),
        };
        let msg = err.to_string();
        assert!(msg.starts_with("stack depth `2` at pc `6` disagrees with predecessor depth `1`"));
        assert!(!msg.ends_with('.'));
    }

    #[test]
    fn index_range_family_display_forms() {
        let cases = [
            (
                CompileError::ConstIndexOutOfRange {
                    pc: 1,
                    index: 3,
                    span: span(0, 1),
                },
                "constant index `3` out of range at pc `1`",
            ),
            (
                CompileError::GlobalIndexOutOfRange {
                    pc: 2,
                    index: 0,
                    span: span(0, 1),
                },
                "global index `0` out of range at pc `2`",
            ),
            (
                CompileError::LocalSlotOutOfRange {
                    pc: 3,
                    slot: 5,
                    span: span(0, 1),
                },
                "local slot `5` out of range at pc `3`",
            ),
            (
                CompileError::ProtoIndexOutOfRange {
                    pc: 4,
                    index: 1,
                    span: span(0, 1),
                },
                "proto index `1` out of range at pc `4`",
            ),
            (
                CompileError::JumpTargetOutOfRange {
                    pc: 5,
                    target: 8,
                    span: span(0, 1),
                },
                "jump target `8` out of range at pc `5`",
            ),
        ];
        for (err, prefix) in &cases {
            let msg = err.to_string();
            assert!(
                msg.starts_with(prefix),
                "message `{msg}` must start with `{prefix}`"
            );
            assert!(!msg.ends_with('.'), "no trailing period in `{msg}`");
        }
    }

    #[test]
    fn spanless_variants_report_structural_position() {
        let missing = CompileError::DebugInfoMissing { func: 2, pc: 4 };
        assert_eq!(
            missing.to_string(),
            "debug info missing for func `2` pc `4` (bare instruction)"
        );
        let term = CompileError::MissingTerminator { func: 1, pc: 6 };
        assert_eq!(
            term.to_string(),
            "func `1` must end with `ret` or `halt` (last pc `6`)"
        );
        let empty = CompileError::EmptyFuncTable;
        assert_eq!(
            empty.to_string(),
            "bytecode has no function proto (entry proto missing)"
        );
    }

    #[test]
    fn halt_outside_entry_display_form() {
        let err = CompileError::HaltOutsideEntry {
            pc: 2,
            span: span(8, 9),
        };
        assert_eq!(
            err.to_string(),
            "`halt` outside entry proto at pc `2` (file#0:8..9)"
        );
    }

    #[test]
    fn variants_carry_pc_depth_and_span_payloads() {
        // Payload assertions: product-side variants keep pc + span,
        // compile-side keep depth + span (FP1 contract).
        let e = CompileError::StackUnderflow {
            pc: 12,
            span: span(4, 6),
        };
        match e {
            CompileError::StackUnderflow {
                pc: 12,
                span: payload,
            } => {
                assert_eq!(payload, span(4, 6));
            }
            other => panic!("wrong variant: {other}"),
        }
        let d = CompileError::DepthExceeded {
            depth: 2000,
            span: span(0, 9),
        };
        assert!(matches!(d, CompileError::DepthExceeded { depth: 2000, .. }));
    }

    #[test]
    fn error_trait_is_implemented_for_boxing() {
        // The 0.8 diagnostics renderer composes error sources; the trait
        // must be implemented even though compile errors have no source
        // chain (inputs are pre-validated).
        let err: Box<dyn std::error::Error> = Box::new(CompileError::EmptyFuncTable);
        assert_eq!(err.to_string(), CompileError::EmptyFuncTable.to_string());
    }
}
