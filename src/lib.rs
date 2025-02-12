pub mod graph;
pub mod query;
pub mod relations;

pub use intuicio_framework_arena::AnyIndex;

#[cfg(test)]
mod tests {
    use crate::{
        graph::Graph,
        query::{Is, Node, Query, Related, Traverse},
    };
    use intuicio_framework_arena::AnyIndex;

    fn is_async<T: Send + Sync>() {}

    struct Child;
    struct Parent;
    struct Effect;
    struct Attribute;

    struct Player;
    struct Tree;
    struct Fire;

    #[derive(Debug)]
    struct Controller {
        forward: bool,
    }

    #[derive(Debug)]
    struct Position(i32, i32);

    #[derive(Debug)]
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

            let name = graph.insert(format!("Tree {}", index));
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
                println!("Player: {} moves forward!", player);
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

        graph.clear();
    }
}
