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
    use super::*;

    #[test]
    fn add_keeps_sorted_dedup() {
        let mut s = ScopeSet::new();
        s.insert(ScopeId(2));
        s.insert(ScopeId(1));
        s.insert(ScopeId(3));
        s.insert(ScopeId(2));
        assert_eq!(
            s.iter().map(|ScopeId(id)| id).collect::<Vec<u32>>(),
            vec![1, 2, 3]
        );
        assert_eq!(s.len(), 3);
    }

    // more ...
}
