use crate::graph::Graph;
use intuicio_data::lifetime::{ValueReadAccess, ValueWriteAccess};
use intuicio_framework_arena::AnyIndex;
use std::marker::PhantomData;

pub struct QueryIter<'a, Fetch: QueryFetch<'a>> {
    access: Fetch::Access,
}

impl<'a, Fetch: QueryFetch<'a>> QueryIter<'a, Fetch> {
    pub fn new(graph: &'a Graph, index: AnyIndex) -> Self {
        Self {
            access: Fetch::access(graph, index),
        }
    }
}

impl<'a, Fetch: QueryFetch<'a>> Iterator for QueryIter<'a, Fetch> {
    type Item = Fetch::Value;

    fn next(&mut self) -> Option<Self::Item> {
        Fetch::fetch(&mut self.access)
    }
}

pub trait QueryFetch<'a> {
    type Value;
    type Access;

    fn access(graph: &'a Graph, index: AnyIndex) -> Self::Access;
    fn fetch(access: &mut Self::Access) -> Option<Self::Value>;
}

pub trait QueryTransform<'a> {
    type Input;
    type Output;

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output>;
}

impl<'a> QueryFetch<'a> for () {
    type Value = ();
    type Access = ();

    fn access(_: &'a Graph, _: AnyIndex) -> Self::Access {}

    fn fetch(_: &mut Self::Access) -> Option<Self::Value> {
        Some(())
    }
}

impl<'a> QueryFetch<'a> for AnyIndex {
    type Value = AnyIndex;
    type Access = AnyIndex;

    fn access(_: &'a Graph, index: AnyIndex) -> Self::Access {
        index
    }

    fn fetch(access: &mut Self::Access) -> Option<Self::Value> {
        Some(*access)
    }
}

pub struct Related<'a, T, Transform: QueryTransform<'a, Input = AnyIndex>>(
    PhantomData<fn() -> &'a (T, Transform)>,
);

impl<'a, T, Transform: QueryTransform<'a, Input = AnyIndex>> QueryFetch<'a>
    for Related<'a, T, Transform>
{
    type Value = Transform::Output;
    type Access = Box<dyn Iterator<Item = Self::Value> + 'a>;

    fn access(graph: &'a Graph, index: AnyIndex) -> Self::Access {
        Box::new(
            graph
                .relations_outgoing::<T>(index)
                .flat_map(|index| Transform::transform(graph, index)),
        )
    }

    fn fetch(access: &mut Self::Access) -> Option<Self::Value> {
        access.next()
    }
}

pub struct Traverse<'a, T, Transform: QueryTransform<'a, Input = AnyIndex>>(
    PhantomData<fn() -> &'a (T, Transform)>,
);

impl<'a, T, Transform: QueryTransform<'a, Input = AnyIndex>> QueryFetch<'a>
    for Traverse<'a, T, Transform>
{
    type Value = Transform::Output;
    type Access = Box<dyn Iterator<Item = Self::Value> + 'a>;

    fn access(graph: &'a Graph, index: AnyIndex) -> Self::Access {
        Box::new(
            graph
                .relations_traverse::<T>(index)
                .flat_map(|index| Transform::transform(graph, index)),
        )
    }

    fn fetch(access: &mut Self::Access) -> Option<Self::Value> {
        access.next()
    }
}

impl<'a> QueryTransform<'a> for AnyIndex {
    type Input = AnyIndex;
    type Output = AnyIndex;

    fn transform(_: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        std::iter::once(input)
    }
}

pub struct Node<'a, T>(PhantomData<fn() -> &'a T>);

impl<'a, T> QueryTransform<'a> for Node<'a, T> {
    type Input = AnyIndex;
    type Output = AnyIndex;

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        graph.is::<T>(input).then_some(input).into_iter()
    }
}

pub struct Is<'a, T>(PhantomData<fn() -> &'a T>);

impl<'a, T> QueryTransform<'a> for Is<'a, T> {
    type Input = AnyIndex;
    type Output = ();

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        graph.is::<T>(input).then_some(()).into_iter()
    }
}

pub struct IsNot<'a, T>(PhantomData<fn() -> &'a T>);

impl<'a, T> QueryTransform<'a> for IsNot<'a, T> {
    type Input = AnyIndex;
    type Output = ();

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        (!graph.is::<T>(input)).then_some(()).into_iter()
    }
}

impl<'a, T> QueryTransform<'a> for &'a T {
    type Input = AnyIndex;
    type Output = ValueReadAccess<'a, T>;

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        graph.read(input).ok().into_iter()
    }
}

impl<'a, T> QueryTransform<'a> for &'a mut T {
    type Input = AnyIndex;
    type Output = ValueWriteAccess<'a, T>;

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        graph.write(input).ok().into_iter()
    }
}

impl<'a, T> QueryTransform<'a> for Option<&'a T> {
    type Input = AnyIndex;
    type Output = Option<ValueReadAccess<'a, T>>;

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        std::iter::once(graph.read(input).ok())
    }
}

impl<'a, T> QueryTransform<'a> for Option<&'a mut T> {
    type Input = AnyIndex;
    type Output = Option<ValueWriteAccess<'a, T>>;

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        std::iter::once(graph.write(input).ok())
    }
}

pub struct Limit<'a, const COUNT: usize, Transform: QueryTransform<'a>>(
    PhantomData<fn() -> &'a Transform>,
);

impl<'a, const COUNT: usize, Transform: QueryTransform<'a>> QueryTransform<'a>
    for Limit<'a, COUNT, Transform>
{
    type Input = Transform::Input;
    type Output = Transform::Output;

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        Transform::transform(graph, input).take(COUNT)
    }
}

pub type Single<'a, Transform> = Limit<'a, 1, Transform>;

pub struct Query<'a, Transform, Fetch>(PhantomData<fn() -> &'a (Transform, Fetch)>)
where
    Transform: QueryTransform<'a, Input = AnyIndex, Output = AnyIndex>,
    Fetch: QueryFetch<'a>;

impl<'a, Transform, Fetch> QueryTransform<'a> for Query<'a, Transform, Fetch>
where
    Transform: QueryTransform<'a, Input = AnyIndex, Output = AnyIndex>,
    Fetch: QueryFetch<'a>,
{
    type Input = AnyIndex;
    type Output = Fetch::Value;

    fn transform(graph: &'a Graph, input: Self::Input) -> impl Iterator<Item = Self::Output> {
        Transform::transform(graph, input).flat_map(|index| graph.query::<Fetch>(index))
    }
}

macro_rules! impl_fetch_tuple {
    ($($type:ident),+) => {
        impl<'a, $($type: QueryFetch<'a>),+> QueryFetch<'a> for ($($type,)+) {
            type Value = ($($type::Value,)+);
            type Access = ($($type::Access,)+);

            fn access(graph: &'a Graph, index: AnyIndex) -> Self::Access {
                ($($type::access(graph, index),)+)
            }

            fn fetch(access: &mut Self::Access) -> Option<Self::Value> {
                #[allow(non_snake_case)]
                let ($($type,)+) = access;
                Some(($($type::fetch($type)?,)+))
            }
        }
    };
}

impl_fetch_tuple!(A);
impl_fetch_tuple!(A, B);
impl_fetch_tuple!(A, B, C);
impl_fetch_tuple!(A, B, C, D);
impl_fetch_tuple!(A, B, C, D, E);
impl_fetch_tuple!(A, B, C, D, E, F);
impl_fetch_tuple!(A, B, C, D, E, F, G);
impl_fetch_tuple!(A, B, C, D, E, F, G, H);
impl_fetch_tuple!(A, B, C, D, E, F, G, H, I);
impl_fetch_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, O);
impl_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, O, P);
