use crate::{
    query::{QueryFetch, QueryIter},
    relations::RelationsTable,
};
use intuicio_data::{
    lifetime::{ValueReadAccess, ValueWriteAccess},
    type_hash::TypeHash,
};
use intuicio_framework_arena::{AnyArena, AnyIndex, ArenaError};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    error::Error,
    marker::PhantomData,
};

#[derive(Default)]
pub struct Graph {
    nodes: AnyArena,
    relations: HashMap<TypeHash, RelationsTable>,
}

impl Graph {
    pub fn with_new_arena_capacity(self, capacity: usize) -> Self {
        Self {
            nodes: AnyArena::default().with_new_arena_capacity(capacity),
            relations: Default::default(),
        }
    }

    pub fn insert<T>(&mut self, value: T) -> AnyIndex {
        self.nodes.insert(value)
    }

    pub fn remove(&mut self, index: AnyIndex) -> Result<(), Box<dyn Error>> {
        self.nodes.remove(index)?;
        for relation in self.relations.values_mut() {
            relation.remove_all(index);
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.relations.clear();
    }

    pub fn is<T>(&self, index: AnyIndex) -> bool {
        // TODO: add `is` test to any arena.
        self.nodes.read::<T>(index).is_ok()
    }

    pub fn read<T>(&self, index: AnyIndex) -> Result<ValueReadAccess<T>, ArenaError> {
        self.nodes.read(index)
    }

    pub fn write<T>(&self, index: AnyIndex) -> Result<ValueWriteAccess<T>, ArenaError> {
        self.nodes.write(index)
    }

    pub fn relate<T>(&mut self, from: AnyIndex, to: AnyIndex) {
        self.relations
            .entry(TypeHash::of::<T>())
            .or_default()
            .insert(from, to);
    }

    pub fn relate_pair<I, O>(&mut self, from: AnyIndex, to: AnyIndex) {
        self.relate::<O>(from, to);
        self.relate::<I>(to, from);
    }

    pub fn unrelate<T>(&mut self, from: AnyIndex, to: AnyIndex) {
        self.relations
            .entry(TypeHash::of::<T>())
            .or_default()
            .remove(from, to);
    }

    pub fn unrelate_pair<I, O>(&mut self, from: AnyIndex, to: AnyIndex) {
        self.unrelate::<O>(from, to);
        self.unrelate::<I>(to, from);
    }

    pub fn unrelate_all<T>(&mut self, from: AnyIndex) {
        self.relations
            .entry(TypeHash::of::<T>())
            .or_default()
            .remove_all(from);
    }

    pub fn are_related<T>(&self, from: AnyIndex, to: AnyIndex) -> bool {
        self.relations
            .get(&TypeHash::of::<T>())
            .map(|relations| relations.contains(from, to))
            .unwrap_or_default()
    }

    pub fn relations_outgoing<T>(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations
            .get(&TypeHash::of::<T>())
            .into_iter()
            .flat_map(move |relations| relations.outgoing(from))
    }

    pub fn relations_incomming<T>(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations
            .get(&TypeHash::of::<T>())
            .into_iter()
            .flat_map(move |relations| relations.incoming(from))
    }

    pub fn relations_traverse<'a, T: 'a>(
        &'a self,
        from: AnyIndex,
    ) -> impl Iterator<Item = AnyIndex> + 'a {
        GraphTraverseIter::<T>::new(self, from)
    }

    pub fn find<R, T>(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations_outgoing::<R>(from)
            .filter(|index| self.is::<T>(*index))
    }

    pub fn query<'a, Fetch: QueryFetch<'a>>(&'a self, index: AnyIndex) -> QueryIter<'a, Fetch> {
        QueryIter::new(self, index)
    }

    pub fn iter<'a, T: 'a>(&'a self) -> impl Iterator<Item = (AnyIndex, ValueReadAccess<'a, T>)> {
        self.nodes.arena::<T>().into_iter().flat_map(|arena| {
            arena
                .indices()
                .map(|index| AnyIndex::new(index, arena.type_hash()))
                .zip(arena.iter::<T>())
        })
    }

    pub fn iter_mut<'a, T: 'a>(
        &'a self,
    ) -> impl Iterator<Item = (AnyIndex, ValueWriteAccess<'a, T>)> {
        self.nodes.arena::<T>().into_iter().flat_map(|arena| {
            arena
                .indices()
                .map(|index| AnyIndex::new(index, arena.type_hash()))
                .zip(arena.iter_mut::<T>())
        })
    }
}

pub struct GraphTraverseIter<'a, T> {
    graph: &'a Graph,
    stack: VecDeque<AnyIndex>,
    visited: HashSet<AnyIndex>,
    _phantom: PhantomData<fn() -> T>,
}

impl<'a, T> GraphTraverseIter<'a, T> {
    fn new(graph: &'a Graph, index: AnyIndex) -> Self {
        Self {
            graph,
            stack: [index].into(),
            visited: Default::default(),
            _phantom: PhantomData,
        }
    }
}

impl<T> Iterator for GraphTraverseIter<'_, T> {
    type Item = AnyIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(index) = self.stack.pop_front() {
            if self.visited.contains(&index) {
                continue;
            }
            self.visited.insert(index);
            for index in self.graph.relations_outgoing::<T>(index) {
                if self.stack.len() == self.stack.capacity() {
                    self.stack.reserve_exact(self.stack.capacity());
                }
                self.stack.push_back(index);
            }
            return Some(index);
        }
        None
    }
}
