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
};

/// A graph data structure that allows for the storage of data in nodes and
/// graph edges in form of relations.
/// Each node can be of any type, and relations use types as categories.
/// Graph supports directed and undirected relations between nodes.
#[derive(Default)]
pub struct Graph {
    pub(crate) nodes: AnyArena,
    pub(crate) relations: HashMap<TypeHash, RelationsTable>,
}

impl Graph {
    /// Creates a new graph with the specified capacity for any new arena created.
    ///
    /// # Arguments
    /// * `capacity` - The capacity of the new arena.
    ///
    /// # Returns
    /// A new `Graph` instance with the specified arena capacity.
    pub fn with_new_arena_capacity(self, capacity: usize) -> Self {
        Self {
            nodes: AnyArena::default().with_new_arena_capacity(capacity),
            relations: Default::default(),
        }
    }

    /// Inserts new node with provided data.
    ///
    /// # Arguments
    /// * `value` - The value to be inserted into the graph.
    ///
    /// # Returns
    /// The index of the newly inserted node.
    pub fn insert<T>(&mut self, value: T) -> AnyIndex {
        self.nodes.insert(value)
    }

    /// Removes node from the graph by its index.
    pub fn remove(&mut self, index: AnyIndex) -> Result<(), Box<dyn Error>> {
        self.nodes.remove(index)?;
        for relation in self.relations.values_mut() {
            relation.remove_all(index);
        }
        Ok(())
    }

    /// Removes all nodes and relations from the graph.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.relations.clear();
    }

    /// Checks if the graph contains a node with the specified index.
    pub fn contains(&self, index: AnyIndex) -> bool {
        self.nodes.contains(index)
    }

    /// Checks if the graph node at the specified index is of the specified type.
    ///
    /// # Arguments
    /// * `index` - The index of the node to check.
    pub fn is<T>(&self, index: AnyIndex) -> bool {
        self.nodes.is::<T>(index).unwrap_or_default()
    }

    /// Returns read access to the node at the specified index.
    ///
    /// # Arguments
    /// * `index` - The index of the node to read.
    ///
    /// # Returns
    /// A `Result` containing the read access to the node or an error.
    pub fn read<T>(&self, index: AnyIndex) -> Result<ValueReadAccess<T>, ArenaError> {
        self.nodes.read(index)
    }

    /// Returns read access to the node at the specified index as a raw pointer.
    ///
    /// # Arguments
    /// * `index` - The index of the node to read.
    ///
    /// # Returns
    /// A `Result` containing the raw pointer to the node or an error.
    ///
    /// # Safety
    /// The caller must ensure that the pointer is used safely and does not lead
    /// to undefined behavior, since they get unrestricted memory block.
    pub unsafe fn read_ptr(&self, index: AnyIndex) -> Result<*const u8, ArenaError> {
        unsafe { self.nodes.read_ptr(index) }
    }

    /// Returns mutable write access to the node at the specified index.
    ///
    /// # Arguments
    /// * `index` - The index of the node to write.
    ///
    /// # Returns
    /// A `Result` containing the write access to the node or an error.
    pub fn write<T>(&self, index: AnyIndex) -> Result<ValueWriteAccess<T>, ArenaError> {
        self.nodes.write(index)
    }

    /// Returns mutable write access to the node at the specified index as a raw pointer.
    ///
    /// # Arguments
    /// * `index` - The index of the node to write.
    ///
    /// # Returns
    /// A `Result` containing the raw pointer to the node or an error.
    ///
    /// # Safety
    /// The caller must ensure that the pointer is used safely and does not lead
    /// to undefined behavior, since they get unrestricted memory block.
    pub unsafe fn write_ptr(&self, index: AnyIndex) -> Result<*mut u8, ArenaError> {
        unsafe { self.nodes.write_ptr(index) }
    }

    /// Relates two nodes with specified relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    /// * `to` - The index of the target node.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    pub fn relate<T>(&mut self, from: AnyIndex, to: AnyIndex) {
        self.relations
            .entry(TypeHash::of::<T>())
            .or_default()
            .insert(from, to);
    }

    /// Relates two nodes with specified relation category in both directions.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    /// * `to` - The index of the target node.
    ///
    /// # Type Parameters
    /// * `I` - The type of the relation category for the target node towards
    ///   source node.
    /// * `O` - The type of the relation category for the source node towards
    ///   target node.
    pub fn relate_pair<I, O>(&mut self, from: AnyIndex, to: AnyIndex) {
        self.relate::<O>(from, to);
        self.relate::<I>(to, from);
    }

    /// Unrelates two nodes with specified relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    /// * `to` - The index of the target node.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    pub fn unrelate<T>(&mut self, from: AnyIndex, to: AnyIndex) {
        self.relations
            .entry(TypeHash::of::<T>())
            .or_default()
            .remove(from, to);
    }

    /// Unrelates two nodes with specified relation category in both directions.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    /// * `to` - The index of the target node.
    ///
    /// # Type Parameters
    /// * `I` - The type of the relation category for the target node towards
    ///   source node.
    /// * `O` - The type of the relation category for the source node towards
    ///   target node.
    pub fn unrelate_pair<I, O>(&mut self, from: AnyIndex, to: AnyIndex) {
        self.unrelate::<O>(from, to);
        self.unrelate::<I>(to, from);
    }

    /// Unrelates all nodes from the specified source node with the specified
    /// relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    pub fn unrelate_all<T>(&mut self, from: AnyIndex) {
        self.relations
            .entry(TypeHash::of::<T>())
            .or_default()
            .remove_all(from);
    }

    /// Checks if two nodes are related with the specified relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    /// * `to` - The index of the target node.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    pub fn are_related<T>(&self, from: AnyIndex, to: AnyIndex) -> bool {
        self.relations
            .get(&TypeHash::of::<T>())
            .map(|relations| relations.contains(from, to))
            .unwrap_or_default()
    }

    /// Gets iterator over all outgoing relations from the specified source node
    /// with the specified relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    ///
    /// # Returns
    /// An iterator over the indices of the target nodes.
    pub fn relations_outgoing<T>(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations_outgoing_raw(from, TypeHash::of::<T>())
    }

    /// Gets iterator over all outgoing relations from the specified source node
    /// with the specified relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    /// * `type_hash` - The type hash of the relation category.
    ///
    /// # Returns
    /// An iterator over the indices of the target nodes.
    pub fn relations_outgoing_raw(
        &self,
        from: AnyIndex,
        type_hash: TypeHash,
    ) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations
            .get(&type_hash)
            .into_iter()
            .flat_map(move |relations| relations.outgoing(from))
    }

    /// Gets iterator over all outgoing relations from the specified source node
    /// with any relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    ///
    /// # Returns
    /// An iterator over the indices of the target nodes.
    pub fn relations_outgoing_any(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations
            .values()
            .flat_map(move |relations| relations.outgoing(from))
    }

    /// Gets iterator over incoming relations to the specified target node
    /// with the specified relation category.
    ///
    /// # Arguments
    /// * `to` - The index of the target node.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    ///
    /// # Returns
    /// An iterator over the indices of the source nodes.
    pub fn relations_incomming<T>(&self, to: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations_incomming_raw(to, TypeHash::of::<T>())
    }

    /// Gets iterator over incoming relations to the specified target node
    /// with the specified relation category.
    ///
    /// # Arguments
    /// * `to` - The index of the target node.
    /// * `type_hash` - The type hash of the relation category.
    ///
    /// # Returns
    /// An iterator over the indices of the source nodes.
    pub fn relations_incomming_raw(
        &self,
        to: AnyIndex,
        type_hash: TypeHash,
    ) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations
            .get(&type_hash)
            .into_iter()
            .flat_map(move |relations| relations.incoming(to))
    }

    /// Gets iterator over incoming relations to the specified target node
    /// with any relation category.
    ///
    /// # Arguments
    /// * `to` - The index of the target node.
    ///
    /// # Returns
    /// An iterator over the indices of the source nodes.
    pub fn relations_incomming_any(&self, to: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations
            .values()
            .flat_map(move |relations| relations.incoming(to))
    }

    /// Gets traverse iterator over all relations from the specified source node
    /// with the specified relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    ///
    /// # Returns
    /// An iterator over the indices of the target nodes.
    pub fn relations_traverse<T>(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        GraphTraverseIter::new::<T>(self, from)
    }

    /// Gets traverse iterator over all relations from the specified source node
    /// with the specified relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    /// * `type_hash` - The type hash of the relation category.
    ///
    /// # Returns
    /// An iterator over the indices of the target nodes.
    pub fn relations_traverse_raw(
        &self,
        from: AnyIndex,
        type_hash: TypeHash,
    ) -> impl Iterator<Item = AnyIndex> + '_ {
        GraphTraverseIter::new_raw(self, from, type_hash)
    }

    /// Gets traverse iterator over all relations from the specified source node
    /// with any relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    ///
    /// # Returns
    /// An iterator over the indices of the target nodes.
    pub fn relations_traverse_any(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        GraphTraverseAnyIter::new(self, from)
    }

    /// Finds all nodes of the specified type that are related to the specified
    /// source node with the specified relation category.
    ///
    /// # Arguments
    /// * `from` - The index of the source node.
    ///
    /// # Type Parameters
    /// * `R` - The type of the relation category.
    /// * `T` - The type of the target nodes.
    ///
    /// # Returns
    /// An iterator over the indices of the target nodes.
    pub fn find<R, T>(&self, from: AnyIndex) -> impl Iterator<Item = AnyIndex> + '_ {
        self.relations_outgoing::<R>(from)
            .filter(|index| self.is::<T>(*index))
    }

    /// Performs query on the graph using the specified index.
    pub fn query<'a, Fetch: QueryFetch<'a>>(&'a self, index: AnyIndex) -> QueryIter<'a, Fetch> {
        QueryIter::new(self, index)
    }

    /// Gets iterator over all nodes of specified type in the graph.
    ///
    /// # Type Parameters
    /// * `T` - The type of the nodes to iterate over.
    ///
    /// # Returns
    /// An iterator over the indices of the nodes and their read access.
    pub fn iter<'a, T: 'a>(&'a self) -> impl Iterator<Item = (AnyIndex, ValueReadAccess<'a, T>)> {
        self.nodes.arena::<T>().into_iter().flat_map(|arena| {
            arena
                .indices()
                .map(|index| AnyIndex::new(index, arena.type_hash()))
                .zip(arena.iter::<T>())
        })
    }

    /// Gets iterator over all nodes of specified type in the graph with mutable access.
    ///
    /// # Type Parameters
    /// * `T` - The type of the nodes to iterate over.
    ///
    /// # Returns
    /// An iterator over the indices of the nodes and their mutable write access.
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

    /// Gets iterator over all node indices in the graph.
    ///
    /// # Returns
    /// An iterator over the indices of the nodes.
    pub fn indices(&self) -> impl Iterator<Item = AnyIndex> + '_ {
        self.nodes.indices()
    }

    /// Gets iterator over all relations in the graph.
    ///
    /// # Returns
    /// An iterator over tuples containing the type hash and the indices of the
    /// related nodes.
    pub fn relations(&self) -> impl Iterator<Item = (TypeHash, AnyIndex, AnyIndex)> + '_ {
        self.relations.iter().flat_map(|(type_hash, relations)| {
            relations.iter().map(|(from, to)| (*type_hash, from, to))
        })
    }

    /// Finds all cycles in the graph for the specified relation category.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    ///
    /// # Returns
    /// An iterator over list of indices representing the cycle path.
    pub fn find_cycles<T>(&self) -> impl Iterator<Item = Vec<AnyIndex>> + '_ {
        self.nodes.indices().filter_map(|index| {
            let cycle = self.find_cycle::<T>(index);
            if cycle.is_empty() { None } else { Some(cycle) }
        })
    }

    /// Finds a cycle in the graph starting from the specified index.
    ///
    /// # Arguments
    /// * `index` - The index to start searching for a cycle.
    ///
    /// # Type Parameters
    /// * `T` - The type of the relation category.
    ///
    /// # Returns
    /// A list of indices representing the cycle path.
    pub fn find_cycle<T>(&self, index: AnyIndex) -> Vec<AnyIndex> {
        fn walk<T>(
            visited: &mut HashSet<AnyIndex>,
            path: &mut Vec<AnyIndex>,
            graph: &Graph,
            source: AnyIndex,
        ) -> Option<usize> {
            if visited.contains(&source) {
                return None;
            }
            visited.insert(source);
            path.push(source);
            for target in graph.relations_outgoing::<T>(source) {
                if let Some(index) = path.iter().position(|item| *item == target) {
                    return Some(index);
                }
                if let Some(index) = walk::<T>(visited, path, graph, target) {
                    return Some(index);
                }
            }
            path.pop();
            None
        }

        let mut visited = HashSet::default();
        let mut path = Vec::default();
        if let Some(index) = walk::<T>(&mut visited, &mut path, self, index) {
            return path[index..].to_vec();
        }
        path
    }
}

pub struct GraphTraverseIter<'a> {
    graph: &'a Graph,
    stack: VecDeque<AnyIndex>,
    visited: HashSet<AnyIndex>,
    type_hash: TypeHash,
}

impl<'a> GraphTraverseIter<'a> {
    fn new<T>(graph: &'a Graph, index: AnyIndex) -> Self {
        Self::new_raw(graph, index, TypeHash::of::<T>())
    }

    fn new_raw(graph: &'a Graph, index: AnyIndex, type_hash: TypeHash) -> Self {
        Self {
            graph,
            stack: [index].into(),
            visited: Default::default(),
            type_hash,
        }
    }
}

impl Iterator for GraphTraverseIter<'_> {
    type Item = AnyIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(index) = self.stack.pop_front() {
            if self.visited.contains(&index) {
                continue;
            }
            self.visited.insert(index);
            for index in self.graph.relations_outgoing_raw(index, self.type_hash) {
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

pub struct GraphTraverseAnyIter<'a> {
    graph: &'a Graph,
    stack: VecDeque<AnyIndex>,
    visited: HashSet<AnyIndex>,
}

impl<'a> GraphTraverseAnyIter<'a> {
    fn new(graph: &'a Graph, index: AnyIndex) -> Self {
        Self {
            graph,
            stack: [index].into(),
            visited: Default::default(),
        }
    }
}

impl Iterator for GraphTraverseAnyIter<'_> {
    type Item = AnyIndex;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(index) = self.stack.pop_front() {
            if self.visited.contains(&index) {
                continue;
            }
            self.visited.insert(index);
            for index in self.graph.relations_outgoing_any(index) {
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
