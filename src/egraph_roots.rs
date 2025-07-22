use std::fmt::Display;
use egg::{Analysis, EGraph, Id, Language};
use crate::{egg_to_serialized_egraph, SerializedEGraph};

#[derive(Clone, Debug)]
pub struct EGraphRoots<L: Language, N: Analysis<L, Data: Clone>> {
    pub egraph: EGraph<L, N>,
    pub roots: Vec<Id>,
}

impl<L, N> Default for EGraphRoots<L, N>
where
    L: Language,
    N: Analysis<L> + Default,
    N::Data: Clone,
{
    fn default() -> Self {
        Self {
            egraph: Default::default(),
            roots: Default::default(),
        }
    }
}

impl<L, N> From<&EGraphRoots<L, N>> for SerializedEGraph 
where 
    L: Language + Display,
    N: Analysis<L>,
    N::Data: Clone,
{
    fn from(er: &EGraphRoots<L, N>) -> Self {
        egg_to_serialized_egraph(&er.egraph, &er.roots)
    }
}