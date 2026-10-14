use indexmap::IndexSet;
use intuicio_framework_arena::AnyIndex;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct RelationsTable {
    outgoing: HashMap<AnyIndex, IndexSet<AnyIndex>>,
    incoming: HashMap<AnyIndex, IndexSet<AnyIndex>>,
}

impl RelationsTable {
    pub(crate) fn insert(&mut self, from: AnyIndex, to: AnyIndex) {
        self.outgoing.entry(from).or_default().insert(to);
        self.incoming.entry(to).or_default().insert(from);
    }

    /// Inserts the relation at `index` in the outgoing order of `from`.
    /// An existing relation moves to `index`. The index is clamped to the set length.
    pub(crate) fn insert_at(&mut self, from: AnyIndex, to: AnyIndex, index: usize) {
        let set = self.outgoing.entry(from).or_default();
        set.shift_remove(&to);
        let index = index.min(set.len());
        set.shift_insert(index, to);
        self.incoming.entry(to).or_default().insert(from);
    }

    pub(crate) fn index_of(&self, from: AnyIndex, to: AnyIndex) -> Option<usize> {
        self.outgoing.get(&from)?.get_index_of(&to)
    }

    pub(crate) fn remove(&mut self, from: AnyIndex, to: AnyIndex) {
        if let Some(set) = self.outgoing.get_mut(&from) {
            set.shift_remove(&to);
            if set.is_empty() {
                self.outgoing.remove(&from);
            }
        }
        if let Some(set) = self.incoming.get_mut(&to) {
            set.shift_remove(&from);
            if set.is_empty() {
                self.incoming.remove(&to);
            }
        }
    }

    pub(crate) fn remove_all(&mut self, from: AnyIndex) {
        if let Some(set) = self.outgoing.remove(&from) {
            for to in set {
                if let Some(set) = self.incoming.get_mut(&to) {
                    set.shift_remove(&from);
                    if set.is_empty() {
                        self.incoming.remove(&to);
                    }
                }
            }
        }
    }

    /// Removes every relation that has `index` on either side.
    pub(crate) fn remove_node(&mut self, index: AnyIndex) {
        self.remove_all(index);
        if let Some(set) = self.incoming.remove(&index) {
            for from in set {
                if let Some(set) = self.outgoing.get_mut(&from) {
                    set.shift_remove(&index);
                    if set.is_empty() {
                        self.outgoing.remove(&from);
                    }
                }
            }
        }
    }

    pub(crate) fn contains(&self, from: AnyIndex, to: AnyIndex) -> bool {
        self.outgoing
            .get(&from)
            .map(|set| set.contains(&to))
            .unwrap_or_default()
    }

    pub(crate) fn outgoing(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.outgoing
            .get(&from)
            .into_iter()
            .flat_map(|set| set.iter().copied())
    }

    pub(crate) fn incoming(&self, to: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.incoming
            .get(&to)
            .into_iter()
            .flat_map(|set| set.iter().copied())
    }

    pub(crate) fn iter_outgoing(&self) -> impl Iterator<Item = (AnyIndex, AnyIndex)> + '_ {
        self.outgoing
            .iter()
            .flat_map(|(from, set)| set.iter().map(move |to| (*from, *to)))
    }

    pub(crate) fn iter_incoming(&self) -> impl Iterator<Item = (AnyIndex, AnyIndex)> + '_ {
        self.incoming
            .iter()
            .flat_map(|(to, set)| set.iter().map(move |from| (*from, *to)))
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.outgoing.is_empty() && self.incoming.is_empty()
    }
}
