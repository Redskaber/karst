//! @path: karst/crates/karst_expander/ctx.rs
//! @author: redskaber
//! @datetime: 2026-10-04
//! @discription: karst::crates::karst_expander::ctx
//!
//! ExpandCtx - the expansion environment (scope frames + work budget)

use std::collections::HashSet;

use karst_span::span::Span;
use karst_syntax::{ScopeId, ScopeSet};

use crate::ExpandError;
use crate::STAGE0_GLOBALS;

pub const EXPANSION_LIMIT: usize = 512;

/// The outcome of scope-keyed identifiter resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Resolution {
    Local,
    Global,
    Unbound,
}

/// One lexical frame
pub(crate) struct Frame {
    scope: ScopeId,
    names: HashSet<String>,
}

/// The expansion environment carried through one `expand_program` or `expand` call tree.
#[derive(Default)]
pub struct ExpandCtx {
    frames: Vec<Frame>,
    next_scope: u32,
    depth: usize,
}

impl ExpandCtx {
    pub fn new() -> ExpandCtx {
        ExpandCtx {
            frames: Vec::new(),
            next_scope: 0,
            depth: 0,
        }
    }

    /// Allocate the next fresh scope
    pub(crate) fn fresh_scope(&mut self) -> ScopeId {
        let scope = ScopeId(self.next_scope);
        self.next_scope += 1;
        scope
    }

    /// Push a lexical frame for `scope`
    pub(crate) fn push_frame(&mut self, scope_id: ScopeId) {
        self.frames.push(Frame {
            scope: scope_id,
            names: HashSet::new(),
        });
    }

    /// Pop the innermost lexical frame
    pub(crate) fn pop_frame(&mut self) {
        self.frames.pop();
    }

    /// bind `name` in the innermost frame
    pub(crate) fn bind(&mut self, name: &str) {
        if let Some(frame) = self.frames.last_mut() {
            frame.names.insert(name.to_owned());
        }
    }

    /// Enter one expansion level
    pub(crate) fn enter(&mut self, span: Span) -> Result<(), ExpandError> {
        if self.depth >= EXPANSION_LIMIT {
            return Err(ExpandError::ExpansionDepthExceeded {
                limit: EXPANSION_LIMIT,
                span,
            });
        }
        self.depth += 1;
        Ok(())
    }

    /// Leave one expansion level (paired with [`Self::enter`]).
    pub(crate) fn exit(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Resolve `name` against `scopes`
    pub(crate) fn resolve(&self, name: &str, scopes: &ScopeSet) -> Resolution {
        for frame in self.frames.iter().rev() {
            if frame.names.contains(name) && scopes.contains(frame.scope) {
                return Resolution::Local;
            }
        }
        if STAGE0_GLOBALS.contains(&name) {
            return Resolution::Global;
        }
        Resolution::Unbound
    }

    /// Module isolation, the caller restores them with [`Self::restore_frames`]
    pub(crate) fn isolate_frames(&mut self) -> Vec<Frame> {
        std::mem::take(&mut self.frames)
    }

    /// Restore frames detached by [`Self::isolate_frames`]
    pub(crate) fn restore_frames(&mut self, saved: Vec<Frame>) {
        self.frames = saved;
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the ExpandCtx resolution and budget machinery
    //! (sub-stage test doc FP4).

    use super::*;

    fn scopes_of(ids: &[u32]) -> ScopeSet {
        let mut set = ScopeSet::new();
        for id in ids {
            set.insert(ScopeId(*id));
        }
        set
    }

    // ---------- positive ----------

    #[test]
    fn fresh_scope_is_monotonic() {
        let mut ctx = ExpandCtx::new();
        let a = ctx.fresh_scope();
        let b = ctx.fresh_scope();
        let c = ctx.fresh_scope();
        assert!(a.0 < b.0 && b.0 < c.0, "scope ids must be monotonic");
    }

    #[test]
    fn resolve_local_and_global_fallback() {
        let mut ctx = ExpandCtx::new();
        let scope = ctx.fresh_scope();
        ctx.push_frame(scope);
        ctx.bind("x");
        // Colored identifier: its scope set contains the frame scope.
        assert_eq!(ctx.resolve("x", &scopes_of(&[scope.0])), Resolution::Local);
        // Flat global fallback for the Stage 0 surface.
        assert_eq!(ctx.resolve("+", &scopes_of(&[])), Resolution::Global);
        ctx.pop_frame();
    }

    #[test]
    fn progressive_binding_gives_let_star_visibility() {
        let mut ctx = ExpandCtx::new();
        let scope = ctx.fresh_scope();
        ctx.push_frame(scope);
        // Before binding: unseen even when colored.
        assert_eq!(
            ctx.resolve("y", &scopes_of(&[scope.0])),
            Resolution::Unbound
        );
        ctx.bind("y");
        // After binding: visible to later (colored) expressions only.
        assert_eq!(ctx.resolve("y", &scopes_of(&[scope.0])), Resolution::Local);
        ctx.pop_frame();
    }

    // ---------- negative ----------

    #[test]
    fn uncolored_reference_does_not_resolve() {
        let mut ctx = ExpandCtx::new();
        let scope = ctx.fresh_scope();
        ctx.push_frame(scope);
        ctx.bind("x");
        // Empty scope set (hand-built tree, never colored): the frame
        // scope is not a member, so the binding is invisible — the
        // scope-keyed rule fails closed instead of matching by name.
        assert_eq!(ctx.resolve("x", &scopes_of(&[])), Resolution::Unbound);
        ctx.pop_frame();
    }

    #[test]
    fn popped_frame_binding_is_gone() {
        let mut ctx = ExpandCtx::new();
        let scope = ctx.fresh_scope();
        ctx.push_frame(scope);
        ctx.bind("x");
        ctx.pop_frame();
        assert_eq!(
            ctx.resolve("x", &scopes_of(&[scope.0])),
            Resolution::Unbound
        );
    }

    #[test]
    fn unknown_name_is_unbound() {
        let ctx = ExpandCtx::new();
        assert_eq!(ctx.resolve("nope", &scopes_of(&[])), Resolution::Unbound);
    }

    #[test]
    fn name_bound_at_foreign_scope_stays_unbound() {
        // Scope-keyed precision: the name exists in the environment,
        // but at a scope the identifier does not carry — the hygiene
        // rule refuses name-only matches (docs/stage0.md §19.2
        // invariant 2 skeleton).
        let mut ctx = ExpandCtx::new();
        let scope_a = ctx.fresh_scope();
        let scope_b = ctx.fresh_scope();
        ctx.push_frame(scope_a);
        ctx.bind("x");
        // Identifier colored with scope B only: the binding at scope A
        // must not resolve.
        assert_eq!(
            ctx.resolve("x", &scopes_of(&[scope_b.0])),
            Resolution::Unbound
        );
        // The same identifier carrying scope A resolves — the
        // membership rule, not the name, decides.
        assert_eq!(
            ctx.resolve("x", &scopes_of(&[scope_a.0])),
            Resolution::Local
        );
        ctx.pop_frame();
    }

    #[test]
    fn stray_exit_does_not_underflow_the_budget() {
        // Saturating discipline: a stray exit below zero must not
        // underflow (and must not silently grant budget).
        let mut ctx = ExpandCtx::new();
        ctx.exit();
        ctx.exit();
        let span = karst_span::Span::root(
            karst_span::FileId(0),
            karst_span::ByteOffset(0),
            karst_span::ByteOffset(1),
        )
        .expect("fixture span");
        // Budget accounting stays intact: the full limit is still
        // consumable, and exhaustion still rejects.
        for _ in 0..EXPANSION_LIMIT {
            ctx.enter(span).expect("budget must stay intact");
        }
        assert!(ctx.enter(span).is_err());
    }

    #[test]
    fn isolate_frames_hides_outer_bindings() {
        let mut ctx = ExpandCtx::new();
        let scope = ctx.fresh_scope();
        ctx.push_frame(scope);
        ctx.bind("outer");
        let saved = ctx.isolate_frames();
        assert_eq!(
            ctx.resolve("outer", &scopes_of(&[scope.0])),
            Resolution::Unbound
        );
        ctx.restore_frames(saved);
        assert_eq!(
            ctx.resolve("outer", &scopes_of(&[scope.0])),
            Resolution::Local
        );
    }

    #[test]
    fn work_budget_rejects_beyond_the_limit() {
        let span = karst_span::Span::root(
            karst_span::FileId(0),
            karst_span::ByteOffset(0),
            karst_span::ByteOffset(1),
        )
        .expect("fixture span");
        let mut ctx = ExpandCtx::new();
        for _ in 0..EXPANSION_LIMIT {
            ctx.enter(span).expect("within budget must pass");
        }
        // One more level: beyond the budget — structured rejection.
        let err = ctx.enter(span).expect_err("over budget must fail");
        assert_eq!(
            err,
            ExpandError::ExpansionDepthExceeded {
                limit: EXPANSION_LIMIT,
                span
            }
        );
        // exit restores the budget (paired accounting).
        ctx.exit();
        ctx.enter(span).expect("exit must release one budget unit");
    }
}
