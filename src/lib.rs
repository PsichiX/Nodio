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
    use intuicio_core::prelude::*;
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
}
