use intuicio_framework_arena::AnyIndex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct RelationsTable {
    outgoing: HashMap<AnyIndex, HashSet<AnyIndex>>,
    incoming: HashMap<AnyIndex, HashSet<AnyIndex>>,
}

impl RelationsTable {
    pub(crate) fn insert(&mut self, from: AnyIndex, to: AnyIndex) {
        self.outgoing.entry(from).or_default().insert(to);
        self.incoming.entry(to).or_default().insert(from);
    }

    pub(crate) fn remove(&mut self, from: AnyIndex, to: AnyIndex) {
        if let Some(set) = self.outgoing.get_mut(&from) {
            set.remove(&to);
        }
        if let Some(set) = self.outgoing.get_mut(&to) {
            set.remove(&from);
        }
    }

    pub(crate) fn remove_all(&mut self, from: AnyIndex) {
        if let Some(set) = self.outgoing.get_mut(&from) {
            for to in set.drain() {
                if let Some(set) = self.incoming.get_mut(&to) {
                    set.remove(&from);
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

    pub(crate) fn iter(&self) -> impl Iterator<Item = (AnyIndex, AnyIndex)> + '_ {
        self.outgoing
            .iter()
            .flat_map(|(from, set)| set.iter().map(move |to| (*from, *to)))
    }
}
