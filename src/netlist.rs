use egg::{Analysis, EGraph, Language};
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use crate::egraph_roots::EGraphRoots;

#[derive(Debug, Clone)]
pub struct Netlist<N, E> {
    pub graph: Graph<N, E>,
    pub roots: Vec<NodeIndex>,
    pub leaves: Vec<NodeIndex>,
}

impl<N, E> Default for Netlist<N, E>
{
    fn default() -> Self {
        Self {
            graph: Default::default(),
            roots: Default::default(),
            leaves: Default::default(),
        }
    }
}
