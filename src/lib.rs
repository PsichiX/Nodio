pub mod graph;
pub mod prefab;
pub mod query;

mod relations;

pub use intuicio_data::lifetime::{ValueReadAccess, ValueWriteAccess};
pub use intuicio_framework_arena::AnyIndex;

pub mod third_party {
    pub use intuicio_core;
    pub use intuicio_data;
    pub use intuicio_derive;
    pub use intuicio_framework_arena;
    pub use intuicio_framework_serde;
}

#[cfg(test)]
mod tests {
    use crate::{
        graph::Graph,
        prefab::Prefab,
        query::{Is, Node, Query, Related, Traverse},
    };
    use intuicio_core::{registry::Registry, types::struct_type::NativeStructBuilder};
    use intuicio_framework_arena::AnyIndex;
    use intuicio_framework_serde::SerializationRegistry;
    use serde::{Deserialize, Serialize};

    fn is_async<T: Send + Sync>() {}

    #[derive(Default, Serialize, Deserialize)]
    struct Parent;

    #[derive(Default, Serialize, Deserialize)]
    struct Child;

    #[derive(Default, Serialize, Deserialize)]
    struct Effect;

    #[derive(Default, Serialize, Deserialize)]
    struct Attribute;

    #[derive(Default, Serialize, Deserialize)]
    struct Player;

    #[derive(Default, Serialize, Deserialize)]
    struct Tree;

    #[derive(Default, Serialize, Deserialize)]
    struct Fire;

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct Controller {
        forward: bool,
    }

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct Position(i32, i32);

    #[derive(Debug, Default, Serialize, Deserialize)]
    struct Health(usize);

    #[test]
    fn test_graph() {
        is_async::<Graph>();

        let mut graph = Graph::default();
        let root = graph.insert(());
        let fire = graph.insert(Fire);

        let player = graph.insert(Player);
        graph.relate_pair::<Parent, Child>(root, player);

        let name = graph.insert("Player".to_owned());
        graph.relate::<Attribute>(player, name);

        let controller = graph.insert(Controller { forward: true });
        graph.relate_pair::<Parent, Child>(player, controller);

        let name = graph.insert("Player controller".to_owned());
        graph.relate::<Attribute>(controller, name);

        let position = graph.insert(Position(0, 0));
        graph.relate_pair::<Parent, Child>(player, position);

        for index in 0..5 {
            let tree = graph.insert(Tree);
            graph.relate_pair::<Parent, Child>(root, tree);

            let name = graph.insert(format!("Tree {index}"));
            graph.relate::<Attribute>(player, name);

            let health = graph.insert(Health(2));
            graph.relate_pair::<Parent, Child>(tree, health);

            if index % 2 == 0 {
                graph.relate::<Effect>(tree, fire);
            }
        }

        for (controller, mut position) in
            graph.query::<(Related<Child, &Controller>, Related<Child, &mut Position>)>(player)
        {
            if controller.forward {
                println!("Player: {player} moves forward!");
                position.0 += 1;
                position.1 += 2;
            }
        }

        for (index, mut health, _) in graph.query::<Related<
            Child,
            Query<
                Node<Tree>,
                (
                    AnyIndex,
                    Related<Child, &mut Health>,
                    Related<Effect, Is<Fire>>,
                ),
            >,
        >>(root)
        {
            health.0 = health.0.saturating_sub(1);
            println!("Tree: {} got fire damage! Health: {}", index, health.0);
        }

        for name in
            graph.query::<Traverse<Child, Query<AnyIndex, Related<Attribute, &String>>>>(root)
        {
            println!("Name: {}", &*name);
        }

        let registry = Registry::default()
            .with_basic_types()
            .with_type(NativeStructBuilder::new::<Parent>().build())
            .with_type(NativeStructBuilder::new::<Child>().build())
            .with_type(NativeStructBuilder::new::<Effect>().build())
            .with_type(NativeStructBuilder::new::<Attribute>().build())
            .with_type(NativeStructBuilder::new::<Player>().build())
            .with_type(NativeStructBuilder::new::<Tree>().build())
            .with_type(NativeStructBuilder::new::<Fire>().build())
            .with_type(NativeStructBuilder::new::<Controller>().build())
            .with_type(NativeStructBuilder::new::<Position>().build())
            .with_type(NativeStructBuilder::new::<Health>().build());

        let serialization = SerializationRegistry::default()
            .with_basic_types()
            .with_serde::<Parent>()
            .with_serde::<Child>()
            .with_serde::<Effect>()
            .with_serde::<Attribute>()
            .with_serde::<Player>()
            .with_serde::<Tree>()
            .with_serde::<Fire>()
            .with_serde::<Controller>()
            .with_serde::<Position>()
            .with_serde::<Health>();

        let prefab = Prefab::from_graph(&graph, &serialization, &registry).unwrap();
        let graph2 = prefab.to_graph(&serialization, &registry).unwrap().0;
        let mut indices = graph.indices().collect::<Vec<_>>();
        let mut indices2 = graph2.indices().collect::<Vec<_>>();
        indices.sort();
        indices2.sort();
        assert_eq!(indices, indices2);
        assert_eq!(graph.relations, graph2.relations);

        graph.clear();
    }

    #[test]
    fn test_cycles() {
        let mut graph = Graph::default();
        let a = graph.insert(());
        let b = graph.insert(());
        let c = graph.insert(());
        let d = graph.insert(());

        graph.relate::<()>(a, b);
        graph.relate::<()>(a, c);
        graph.relate::<()>(b, d);
        graph.relate::<()>(c, d);
        assert!(graph.find_cycles::<()>().next().is_none());

        graph.relate::<()>(d, a);
        assert!(graph.find_cycles::<()>().next().is_some());
    }

    fn children(graph: &Graph, parent: AnyIndex) -> Vec<AnyIndex> {
        graph.relations_outgoing::<Child>(parent).collect()
    }

    fn assert_no_relations_with(graph: &Graph, index: AnyIndex) {
        assert!(
            graph
                .relations()
                .all(|(_, from, to)| from != index && to != index)
        );
        assert!(graph.relations_incomming::<Child>(index).next().is_none());
        assert!(graph.relations_outgoing::<Child>(index).next().is_none());
    }

    #[test]
    fn test_order_after_insert() {
        let mut graph = Graph::default();
        let root = graph.insert(());
        let nodes = (0..8).map(|_| graph.insert(())).collect::<Vec<_>>();
        for node in &nodes {
            graph.relate::<Child>(root, *node);
        }
        assert_eq!(children(&graph, root), nodes);

        let first = graph.insert(());
        graph.relate_at::<Child>(root, first, 0);
        let middle = graph.insert(());
        graph.relate_at::<Child>(root, middle, 4);
        let last = graph.insert(());
        graph.relate_at::<Child>(root, last, 100);

        let mut expected = nodes.clone();
        expected.insert(0, first);
        expected.insert(4, middle);
        expected.push(last);
        assert_eq!(children(&graph, root), expected);
        assert_eq!(graph.relation_index::<Child>(root, middle), Some(4));
        assert_eq!(graph.relation_index::<Child>(middle, root), None);
    }

    #[test]
    fn test_order_after_remove() {
        let mut graph = Graph::default();
        let root = graph.insert(());
        let nodes = (0..6).map(|_| graph.insert(())).collect::<Vec<_>>();
        for node in &nodes {
            graph.relate_pair::<Parent, Child>(root, *node);
        }

        graph.unrelate::<Child>(root, nodes[1]);
        graph.remove(nodes[3]).unwrap();
        assert_eq!(
            children(&graph, root),
            vec![nodes[0], nodes[2], nodes[4], nodes[5]]
        );
        assert!(
            graph
                .relations_incomming::<Child>(nodes[1])
                .next()
                .is_none()
        );
        assert_no_relations_with(&graph, nodes[3]);
        assert!(
            graph
                .relations_outgoing::<Parent>(nodes[3])
                .next()
                .is_none()
        );
        assert!(
            graph
                .relations_incomming::<Parent>(root)
                .all(|index| index != nodes[3])
        );
    }

    #[test]
    fn test_order_after_move() {
        let mut graph = Graph::default();
        let root = graph.insert(());
        let nodes = (0..5).map(|_| graph.insert(())).collect::<Vec<_>>();
        for node in &nodes {
            graph.relate::<Child>(root, *node);
        }

        graph.move_relation::<Child>(root, nodes[4], 0);
        graph.move_relation::<Child>(root, nodes[1], 3);
        assert_eq!(
            children(&graph, root),
            vec![nodes[4], nodes[0], nodes[2], nodes[1], nodes[3]]
        );

        let stranger = graph.insert(());
        graph.move_relation::<Child>(root, stranger, 0);
        assert!(!graph.are_related::<Child>(root, stranger));

        graph.relate_at::<Child>(root, nodes[3], 1);
        assert_eq!(
            children(&graph, root),
            vec![nodes[4], nodes[3], nodes[0], nodes[2], nodes[1]]
        );
        assert_eq!(
            graph
                .relations_incomming::<Child>(nodes[3])
                .collect::<Vec<_>>(),
            vec![root]
        );
    }

    #[test]
    fn test_unrelate_leaves_no_stale_incoming() {
        let mut graph = Graph::default();
        let a = graph.insert(());
        let b = graph.insert(());
        graph.relate::<Child>(a, b);
        graph.unrelate::<Child>(a, b);
        assert!(graph.relations_incomming::<Child>(b).next().is_none());
        assert!(graph.relations_outgoing::<Child>(a).next().is_none());
        assert!(graph.relations.values().all(|table| table.is_empty()));
    }

    #[test]
    fn test_remove_leaves_no_stale_relations() {
        let mut graph = Graph::default();
        let parent = graph.insert(());
        let node = graph.insert(());
        let child = graph.insert(());
        graph.relate_pair::<Parent, Child>(parent, node);
        graph.relate_pair::<Parent, Child>(node, child);
        graph.relate::<Effect>(child, node);

        graph.remove(node).unwrap();
        assert_no_relations_with(&graph, node);
        assert!(children(&graph, parent).is_empty());
        assert!(graph.relations_incomming::<Effect>(node).next().is_none());
        assert!(graph.relations_outgoing::<Effect>(child).next().is_none());

        graph.remove(parent).unwrap();
        graph.remove(child).unwrap();
        assert!(graph.relations.values().all(|table| table.is_empty()));
    }

    #[test]
    fn test_prefab_keeps_order() {
        let mut graph = Graph::default();
        let root = graph.insert(Tree);
        let nodes = (0..16).map(|_| graph.insert(Tree)).collect::<Vec<_>>();
        for node in nodes.iter().rev() {
            graph.relate::<Child>(root, *node);
        }
        graph.move_relation::<Child>(root, nodes[7], 0);

        let registry = Registry::default()
            .with_basic_types()
            .with_type(NativeStructBuilder::new::<Child>().build())
            .with_type(NativeStructBuilder::new::<Tree>().build());
        let serialization = SerializationRegistry::default()
            .with_basic_types()
            .with_serde::<Child>()
            .with_serde::<Tree>();

        let prefab = Prefab::from_graph(&graph, &serialization, &registry).unwrap();
        let (graph2, mappings) = prefab.to_graph(&serialization, &registry).unwrap();
        let expected = children(&graph, root)
            .into_iter()
            .map(|index| mappings[&index])
            .collect::<Vec<_>>();
        assert_eq!(children(&graph2, mappings[&root]), expected);
    }
}
