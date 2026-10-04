//! @path: karst/crates/karst_syntax/scope.rs
//! @author: redskaber
//! @datetime: 2026-09-26
//! @discription: karst::crates::karst_syntax::scope

/// scope id
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeId(pub u32);

/// order scope set O(n)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ScopeSet {
    ids: Vec<ScopeId>,
}

impl ScopeSet {
    pub fn new() -> Self {
        ScopeSet { ids: Vec::new() }
    }

    /// insert scope to ScopeSet (binary_search not exist insert)
    pub fn insert(&mut self, scope_id: ScopeId) -> bool {
        match self.ids.binary_search(&scope_id) {
            Ok(_) => false,
            Err(pos) => {
                self.ids.insert(pos, scope_id);
                true
            }
        }
    }

    pub fn contains(&self, scope_id: ScopeId) -> bool {
        self.ids.binary_search(&scope_id).is_ok()
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = ScopeId> + '_ {
        self.ids.iter().copied()
    }
    pub fn union(&self, other: &ScopeSet) -> ScopeSet {
        let mut out = self.clone();
        for id in other.ids.iter().copied() {
            out.insert(id);
        }
        out
    }
    pub fn is_subset_of(&self, other: &ScopeSet) -> bool {
        self.ids.iter().all(|s| other.contains(*s))
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for the ScopeSet feature point (sub-stage test doc FP3).
    use super::*;

    fn set_of(ids: &[u32]) -> ScopeSet {
        let mut set = ScopeSet::new();
        for id in ids {
            set.insert(ScopeId(*id));
        }
        set
    }

    // ---------- positive ----------

    #[test]
    fn insert_contains_iter_are_deterministic() {
        let mut set = ScopeSet::new();
        assert!(set.is_empty());
        // Insertion out of order plus one duplicate.
        assert!(set.insert(ScopeId(3)));
        assert!(set.insert(ScopeId(1)));
        assert!(set.insert(ScopeId(2)));
        assert!(
            !set.insert(ScopeId(3)),
            "duplicate insert must report false"
        );
        assert_eq!(set.len(), 3, "dedup invariant keeps the size at 3");
        for id in [1, 2, 3] {
            assert!(set.contains(ScopeId(id)));
        }
        // Sorted-storage invariant: iteration is ascending.
        let collected: Vec<u32> = set.iter().map(|ScopeId(id)| id).collect();
        assert_eq!(collected, vec![1, 2, 3]);
    }

    #[test]
    fn union_merges_and_subset_relations_hold() {
        let a = set_of(&[0, 1]);
        let b = set_of(&[1, 2]);
        let merged = a.union(&b);
        let collected: Vec<u32> = merged.iter().map(|ScopeId(id)| id).collect();
        assert_eq!(collected, vec![0, 1, 2]);
        // Lexical-resolution query: a binding at {0,1} is visible from a
        // reference at {0,1,2} (subset), and reflexivity holds.
        assert!(a.is_subset_of(&merged));
        assert!(a.is_subset_of(&a));
    }

    // ---------- negative ----------

    #[test]
    fn contains_missing_scope_returns_false() {
        let set = set_of(&[1]);
        assert!(!set.contains(ScopeId(0)));
        assert!(!set.contains(ScopeId(2)));
        assert!(!ScopeSet::new().contains(ScopeId(0)));
    }

    #[test]
    fn non_subset_returns_false() {
        let binding = set_of(&[0, 5]);
        let reference = set_of(&[0, 1, 2]);
        assert!(!binding.is_subset_of(&reference));
    }

    #[test]
    fn different_sets_are_unequal() {
        assert_ne!(set_of(&[0, 1]), set_of(&[0, 2]));
        assert_ne!(set_of(&[0]), ScopeSet::new());
        // Union actually adds members when operands are not nested.
        let a = set_of(&[0]);
        let merged = a.union(&set_of(&[1]));
        assert_ne!(merged, a);
    }

    #[test]
    fn duplicate_union_members_never_appear() {
        let a = set_of(&[0, 1]);
        let double = a.union(&a);
        assert_eq!(double.len(), 2, "union with self must stay deduplicated");
        let collected: Vec<u32> = double.iter().map(|ScopeId(id)| id).collect();
        assert_eq!(collected, vec![0, 1]);
    }
}
