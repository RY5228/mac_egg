use egg::{Analysis, EGraph, Id, Language};

#[derive(Clone)]
pub struct CombinitionalNetlist<L: Language, N: Analysis<L, Data: Clone>> {
    pub egraph: EGraph<L, N>,
    pub roots: Vec<Id>,
}

impl<L, N> Default for CombinitionalNetlist<L, N>
where
    L: Language,
    N: Analysis<L> + Default,
    N::Data: Clone,
{
    fn default() -> Self {
        Self {
            egraph: EGraph::default(),
            roots: vec![],
        }
    }
}
