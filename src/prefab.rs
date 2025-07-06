use crate::{graph::Graph, relations::RelationsTable};
use intuicio_core::{registry::Registry, types::TypeQuery};
use intuicio_data::type_hash::TypeHash;
use intuicio_framework_arena::{AnyArena, AnyIndex, ArenaError, Index};
use intuicio_framework_serde::{Intermediate, SerializationRegistry};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error};

#[derive(Debug)]
pub enum PrefabError {
    CouldNotFindType(TypeHash),
    CouldNotSerializeType {
        type_name: String,
        module_name: Option<String>,
    },
    CouldNotDeserializeType {
        type_name: String,
        module_name: Option<String>,
    },
    Arena(ArenaError),
    Custom(Box<dyn Error>),
}

impl std::fmt::Display for PrefabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CouldNotFindType(type_hash) => {
                write!(f, "Could not find type by hash: {type_hash:?}")
            }
            Self::CouldNotSerializeType {
                type_name,
                module_name,
            } => write!(
                f,
                "Could not serialize type: {}::{}",
                module_name.as_deref().unwrap_or_default(),
                type_name
            ),
            Self::CouldNotDeserializeType {
                type_name,
                module_name,
            } => write!(
                f,
                "Could not deserialize type: {}::{}",
                module_name.as_deref().unwrap_or_default(),
                type_name
            ),
            Self::Arena(error) => write!(f, "Arena: {error}"),
            Self::Custom(error) => write!(f, "Custom: {error}"),
        }
    }
}

impl Error for PrefabError {}

impl From<ArenaError> for PrefabError {
    fn from(error: ArenaError) -> Self {
        Self::Arena(error)
    }
}

impl From<Box<dyn Error>> for PrefabError {
    fn from(error: Box<dyn Error>) -> Self {
        Self::Custom(error)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabDataType {
    pub type_name: String,
    pub module_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabNodesArchetype {
    pub data_type: PrefabDataType,
    pub indices: Vec<Index>,
    pub data: Vec<Intermediate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabRelationsPairItem {
    pub data_type: PrefabDataType,
    pub index: Index,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabRelationsPair {
    pub source: PrefabRelationsPairItem,
    pub target: PrefabRelationsPairItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrefabRelationArchetype {
    pub data_type: PrefabDataType,
    pub pairs: Vec<PrefabRelationsPair>,
}

/// A prefab is a serialized representation of a graph.
/// It contains the nodes and relations of the graph,
/// and can be used to recreate the graph later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prefab {
    pub nodes: Vec<PrefabNodesArchetype>,
    pub relations: Vec<PrefabRelationArchetype>,
}

impl Prefab {
    /// Creates a new prefab from a graph.
    ///
    /// # Arguments
    /// * `graph` - The graph to create the prefab from.
    /// * `serialization` - The serialization registry to use for serialization.
    /// * `registry` - The registry to use for type lookups.
    ///
    /// # Returns
    /// A result containing the prefab or an error.
    pub fn from_graph(
        graph: &Graph,
        serialization: &SerializationRegistry,
        registry: &Registry,
    ) -> Result<Self, PrefabError> {
        let nodes = graph
            .nodes
            .arenas()
            .iter()
            .map(|arena| {
                let type_ = registry
                    .find_type(TypeQuery {
                        type_hash: Some(arena.type_hash()),
                        ..Default::default()
                    })
                    .ok_or_else(|| PrefabError::CouldNotFindType(arena.type_hash()))?;
                let data_type = PrefabDataType {
                    type_name: type_.name().to_owned(),
                    module_name: type_.module_name().map(|name| name.to_owned()),
                };
                let indices = arena.indices().collect::<Vec<_>>();
                let data = indices
                    .iter()
                    .map(|index| unsafe {
                        let data = arena.read_ptr(*index)?;
                        serialization
                            .dynamic_serialize_from(arena.type_hash(), data, registry)
                            .map_err(|_| PrefabError::CouldNotSerializeType {
                                type_name: type_.name().to_owned(),
                                module_name: type_.module_name().map(|name| name.to_owned()),
                            })
                    })
                    .collect::<Result<Vec<_>, PrefabError>>()?;
                Ok(PrefabNodesArchetype {
                    data_type,
                    indices,
                    data,
                })
            })
            .collect::<Result<Vec<_>, PrefabError>>()?;
        let relations = graph
            .relations
            .iter()
            .map(|(type_hash, table)| {
                let type_ = registry
                    .find_type(TypeQuery {
                        type_hash: Some(*type_hash),
                        ..Default::default()
                    })
                    .ok_or_else(|| PrefabError::CouldNotFindType(*type_hash))?;
                let data_type = PrefabDataType {
                    type_name: type_.name().to_owned(),
                    module_name: type_.module_name().map(|name| name.to_owned()),
                };
                let mut pairs = Vec::<PrefabRelationsPair>::default();
                for (source, target) in table.iter() {
                    let source_type = registry
                        .find_type(TypeQuery {
                            type_hash: Some(source.type_hash()),
                            ..Default::default()
                        })
                        .ok_or_else(|| PrefabError::CouldNotFindType(source.type_hash()))?;
                    let source_data_type = PrefabDataType {
                        type_name: source_type.name().to_owned(),
                        module_name: source_type.module_name().map(|name| name.to_owned()),
                    };
                    let target_type = registry
                        .find_type(TypeQuery {
                            type_hash: Some(target.type_hash()),
                            ..Default::default()
                        })
                        .ok_or_else(|| PrefabError::CouldNotFindType(target.type_hash()))?;
                    let target_data_type = PrefabDataType {
                        type_name: target_type.name().to_owned(),
                        module_name: target_type.module_name().map(|name| name.to_owned()),
                    };
                    pairs.push(PrefabRelationsPair {
                        source: PrefabRelationsPairItem {
                            data_type: source_data_type.clone(),
                            index: source.index(),
                        },
                        target: PrefabRelationsPairItem {
                            data_type: target_data_type,
                            index: target.index(),
                        },
                    });
                }
                Ok(PrefabRelationArchetype { data_type, pairs })
            })
            .collect::<Result<Vec<_>, PrefabError>>()?;
        Ok(Self { nodes, relations })
    }

    /// Converts the prefab into a graph.
    ///
    /// # Arguments
    /// * `serialization` - The serialization registry to use for deserialization.
    /// * `registry` - The registry to use for type lookups.
    ///
    /// # Returns
    /// A result containing the graph and a mapping of old indices to new indices.
    pub fn to_graph(
        &self,
        serialization: &SerializationRegistry,
        registry: &Registry,
    ) -> Result<(Graph, HashMap<AnyIndex, AnyIndex>), PrefabError> {
        let mut mappings = HashMap::<AnyIndex, AnyIndex>::default();
        let mut nodes = AnyArena::default();
        for archetype in &self.nodes {
            let type_ = registry
                .find_type(TypeQuery {
                    name: Some(archetype.data_type.type_name.as_str().into()),
                    module_name: archetype
                        .data_type
                        .module_name
                        .as_ref()
                        .map(|name| name.as_str().into()),
                    ..Default::default()
                })
                .ok_or_else(|| PrefabError::CouldNotDeserializeType {
                    type_name: archetype.data_type.type_name.to_owned(),
                    module_name: archetype.data_type.module_name.to_owned(),
                })?;
            unsafe {
                let arena = {
                    nodes.ensure_arena_raw(type_.type_hash(), *type_.layout(), type_.finalizer())
                };
                for (old_index, data) in archetype.indices.iter().zip(archetype.data.iter()) {
                    let (new_index, memory) = arena.allocate();
                    type_.initialize(memory.cast::<_>());
                    serialization
                        .dynamic_deserialize_to(type_.type_hash(), memory, data, true, registry)
                        .map_err(|_| PrefabError::CouldNotDeserializeType {
                            type_name: type_.name().to_owned(),
                            module_name: type_.module_name().map(|name| name.to_owned()),
                        })?;
                    mappings.insert(
                        AnyIndex::new(*old_index, type_.type_hash()),
                        AnyIndex::new(new_index, type_.type_hash()),
                    );
                }
            }
        }
        let relations = self
            .relations
            .iter()
            .map(|archetype| {
                let type_ = registry
                    .find_type(TypeQuery {
                        name: Some(archetype.data_type.type_name.as_str().into()),
                        module_name: archetype
                            .data_type
                            .module_name
                            .as_ref()
                            .map(|name| name.as_str().into()),
                        ..Default::default()
                    })
                    .ok_or_else(|| PrefabError::CouldNotDeserializeType {
                        type_name: archetype.data_type.type_name.to_owned(),
                        module_name: archetype.data_type.module_name.to_owned(),
                    })?;
                let mut table = RelationsTable::default();
                for pair in &archetype.pairs {
                    let source_type = registry
                        .find_type(TypeQuery {
                            name: Some(pair.source.data_type.type_name.as_str().into()),
                            module_name: pair
                                .source
                                .data_type
                                .module_name
                                .as_ref()
                                .map(|name| name.as_str().into()),
                            ..Default::default()
                        })
                        .ok_or_else(|| PrefabError::CouldNotDeserializeType {
                            type_name: pair.source.data_type.type_name.to_owned(),
                            module_name: pair.source.data_type.module_name.to_owned(),
                        })?;
                    let target_type = registry
                        .find_type(TypeQuery {
                            name: Some(pair.target.data_type.type_name.as_str().into()),
                            module_name: pair
                                .target
                                .data_type
                                .module_name
                                .as_ref()
                                .map(|name| name.as_str().into()),
                            ..Default::default()
                        })
                        .ok_or_else(|| PrefabError::CouldNotDeserializeType {
                            type_name: pair.target.data_type.type_name.to_owned(),
                            module_name: pair.target.data_type.module_name.to_owned(),
                        })?;
                    let source = AnyIndex::new(pair.source.index, source_type.type_hash());
                    let source = mappings.get(&source).copied().ok_or_else(|| {
                        PrefabError::Arena(ArenaError::IndexNotFound {
                            type_hash: source.type_hash(),
                            index: source.index(),
                        })
                    })?;
                    let target = AnyIndex::new(pair.target.index, target_type.type_hash());
                    let target = mappings.get(&target).copied().ok_or_else(|| {
                        PrefabError::Arena(ArenaError::IndexNotFound {
                            type_hash: target.type_hash(),
                            index: target.index(),
                        })
                    })?;
                    table.insert(source, target);
                }
                Ok((type_.type_hash(), table))
            })
            .collect::<Result<HashMap<_, _>, PrefabError>>()?;
        Ok((Graph { nodes, relations }, mappings))
    }
}
