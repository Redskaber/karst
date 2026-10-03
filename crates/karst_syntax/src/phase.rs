//! @path: karst/crates/karst_syntax/phase.rs
//! @author: redskaber
//! @datetime: 2026-09-30
//! @discription: karst::crates::karst_syntax::phase
//!
//! Phase levels for syntax objects

use core::fmt;

use crate::SyntaxError;

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
    use super::*;

    #[test]
    fn phase_roundtrip_as_u8_from_u8() {
        for p in [Phase::Runtime, Phase::ExpandTime] {
            let d = Phase::from_u8(p.as_u8()).expect("in-range level must decode");
            assert_eq!(d, p);
        }

        assert_eq!(Phase::Runtime.as_u8(), 0);
        assert_eq!(Phase::ExpandTime.as_u8(), 1);

        let p = Phase::Runtime;
        let q = p;
        assert_eq!(p, q);
    }

    // more ...
}
