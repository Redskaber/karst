//! @path: karst/crates/karst_syntax/scope.rs
//! @author: redskaber
//! @datetime: 2026-09-26
//! @discription: karst::crates::karst_syntax::scope

/// scope id
pub type ScopeId = u32;

/// order scope set O(n)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ScopeSet(Vec<ScopeId>);

impl ScopeSet {
    pub fn new() -> Self {
        ScopeSet(Vec::new())
    }

    /// from iterator build (sort and dedup)
    pub fn from_iter_scopes(scopes: impl IntoIterator<Item = ScopeId>) -> Self {
        let mut v: Vec<ScopeId> = scopes.into_iter().collect();
        v.sort_unstable();
        v.dedup();
        ScopeSet(v)
    }

    /// add scope to ScopeSet (binary_search not exist insert)
    pub fn add(&mut self, scope: ScopeId) {
        match self.0.binary_search(&scope) {
            Ok(_) => {}
            Err(pos) => self.0.insert(pos, scope),
        }
    }

    pub fn remove(&mut self, scope: ScopeId) {
        if let Ok(pos) = self.0.binary_search(&scope) {
            self.0.remove(pos);
        }
    }

    pub fn contains(&self, scope: ScopeId) -> bool {
        self.0.binary_search(&scope).is_ok()
    }

    pub fn union(&self, other: &ScopeSet) -> ScopeSet {
        ScopeSet::from_iter_scopes(self.0.iter().copied().chain(other.0.iter().copied()))
    }

    pub fn is_subset_of(&self, other: &ScopeSet) -> bool {
        self.0.iter().all(|s| other.contains(*s))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = ScopeId> + '_ {
        self.0.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_keeps_sorted_dedup() {
        let mut s = ScopeSet::new();
        s.add(3);
        s.add(1);
        s.add(3);
        s.add(2);
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn union_and_subset() {
        let a = ScopeSet::from_iter_scopes([1, 2]);
        let b = ScopeSet::from_iter_scopes([2, 3]);
        let u = a.union(&b);
        assert_eq!(u.iter().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert!(a.is_subset_of(&u));
        assert!(!u.is_subset_of(&a));
    }

    #[test]
    fn hygiene_invariant_two_scope_sets_never_merge_to_single() {
        let macro_scopes = ScopeSet::from_iter_scopes([10]);
        let user_scopes = ScopeSet::from_iter_scopes([1, 2]);
        let expanded = macro_scopes.union(&user_scopes);
        assert_eq!(expanded.len(), 3);
        assert!(expanded.contains(10) && expanded.contains(1) && expanded.contains(2));
    }
}
