//! @path: karst/crates/karst_syntax/phase.rs
//! @author: redskaber
//! @datetime: 2026-09-30
//! @discription: karst::crates::karst_syntax::phase
//!
//! Phase levels for syntax objects

use std::fmt;

use crate::error::SyntaxError;

/// Phase a syntax object belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// runtime
    Runtime,
    /// expansion macro
    ExpandTime,
}

impl Phase {
    pub fn as_u8(self) -> u8 {
        match self {
            Phase::Runtime => 0,
            Phase::ExpandTime => 1,
        }
    }
    pub fn from_u8(phase: u8) -> Result<Phase, SyntaxError> {
        match phase {
            0 => Ok(Phase::Runtime),
            1 => Ok(Phase::ExpandTime),
            o => Err(SyntaxError::InvalidPhase { phase: o }),
        }
    }
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Phase::Runtime => "runtime",
            Phase::ExpandTime => "expandtime",
        })
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the Phase feature point (sub-stage test doc FP2).
    use super::*;

    // ---------- positive ----------

    #[test]
    fn phase_roundtrip_as_u8_from_u8() {
        for phase in [Phase::Runtime, Phase::ExpandTime] {
            let decoded = Phase::from_u8(phase.as_u8()).expect("in-range level must decode");
            assert_eq!(decoded, phase);
        }
        assert_eq!(Phase::Runtime.as_u8(), 0);
        assert_eq!(Phase::ExpandTime.as_u8(), 1);
        // Copy semantics (cheap to pass through the pipeline).
        let p = Phase::Runtime;
        let q = p;
        assert_eq!(p, q);
    }

    #[test]
    fn phase_display_forms() {
        assert_eq!(Phase::Runtime.to_string(), "runtime");
        assert_eq!(Phase::ExpandTime.to_string(), "expandtime");
    }

    // ---------- negative ----------

    #[test]
    fn from_u8_rejects_two() {
        let err = Phase::from_u8(2).expect_err("level 2 must be rejected");
        assert_eq!(err, SyntaxError::InvalidPhase { phase: 2 });
    }

    #[test]
    fn from_u8_rejects_max() {
        assert!(matches!(
            Phase::from_u8(u8::MAX),
            Err(SyntaxError::InvalidPhase { .. })
        ));
    }

    #[test]
    fn from_u8_error_payload_carries_the_level() {
        let err = Phase::from_u8(7).expect_err("out-of-range level must be rejected");
        let SyntaxError::InvalidPhase { phase } = err else {
            panic!("must be the InvalidPhase variant");
        };
        assert_eq!(phase, 7);
        assert!(err.to_string().contains("`7`"));
    }
}
