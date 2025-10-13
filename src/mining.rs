use crate::SerializedEGraph;
use crate::io::liberty::Library;
use egraph_serialize::ClassId;
use indexmap::IndexMap;
use itertools::{Itertools, sorted};
use libertyparse::PinDirection;
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::prelude::EdgeRef;
use petgraph::visit::{IntoNeighbors, NodeRef, VisitMap, Visitable, depth_first_search};
use petgraph::{Graph, Incoming};
use rustc_hash::FxHashMap;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::iter::zip;
use std::ops::ControlFlow;

/// EClass or ENode (with cell type)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeLabel {
    EClass,
    ENode(String),
}
/// EClass -> ENode: Output pin. ENode -> EClass: Input pin.
pub type EdgeLabel = String;

impl fmt::Display for NodeLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::EClass => write!(f, "Class"),
            Self::ENode(name) => write!(f, "Node_{}", name),
        }
    }
}

struct GSpan {
    egraph: SerializedEGraph,
    library: Library,
    min_support: usize,
    lib_pins: IndexMap<String, (String, Vec<String>)>,
    graph: Graph<NodeLabel, EdgeLabel>,
    frequent_patterns: Vec<DFSCode>,
}

type PatternId = usize;
type NodeId = egraph_serialize::NodeId;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DFSEdge {
    pub i: PatternId,
    pub j: PatternId,
    pub label_i: NodeLabel,
    pub label_ij: String,
    pub label_j: NodeLabel,
}

// impl DFSEdge {
//     pub fn new(
//         i: PatternId,
//         j: PatternId,
//         label_i: NodeLabel,
//         label_ij: String,
//         label_j: NodeLabel,
//     ) -> Self {
//         Self {
//             i,
//             j,
//             label_i,
//             label_ij,
//             label_j,
//         }
//     }
// }

macro_rules! dfs_edge {
    ($from:literal, $to:literal, $class:expr, $label:literal, $node:ident($inner:literal)) => {
        DFSEdge {
            i: $from,
            j: $to,
            label_i: $class,
            label_ij: $label.into(),
            label_j: $node($inner.into()),
        }
    };

    ($from:literal, $to:literal, $node:ident($inner:literal), $label:literal, $class:expr) => {
        DFSEdge {
            i: $from,
            j: $to,
            label_i: $node($inner.into()),
            label_ij: $label.into(),
            label_j: $class,
        }
    };

    ($from:literal, $to:literal, $label_i:expr, $label_ij:expr, $label_j:expr) => {
        DFSEdge {
            i: $from,
            j: $to,
            label_i: $label_i,
            label_ij: $label_ij,
            label_j: $label_j,
        }
    };
}

impl fmt::Display for DFSEdge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({},{},{},{},{})",
            self.i, self.j, self.label_i, self.label_ij, self.label_j
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DFSCode {
    pub edges: Vec<DFSEdge>,
}

impl DFSCode {
    pub fn to_graph(&self) -> Graph<NodeLabel, EdgeLabel> {
        use std::collections::hash_map::Entry;
        let mut graph = Graph::new();
        let mut node_map = FxHashMap::default();
        for edge in &self.edges {
            let from = match node_map.entry(edge.i) {
                Entry::Vacant(e) => {
                    let nid = graph.add_node(edge.label_i.clone());
                    e.insert(nid);
                    nid
                }
                Entry::Occupied(e) => *e.get(),
            };
            let to = match node_map.entry(edge.j) {
                Entry::Vacant(e) => {
                    let nid = graph.add_node(edge.label_j.clone());
                    e.insert(nid);
                    nid
                }
                Entry::Occupied(e) => *e.get(),
            };
            graph.add_edge(
                from,
                to,
                edge.label_ij.clone(),
                // (edge.label_ij.0.as_str(), edge.label_ij.1.as_str()),
            );
        }
        graph
    }

    pub fn is_min(&self) -> bool {
        if self.edges.len() == 1 {
            return true;
        }
        let graph = self.to_graph();
        // ensure DAG
        toposort(&graph, None).unwrap();
        let roots = graph
            .node_indices()
            .filter(|&n| graph.neighbors_directed(n, Incoming).count() == 0)
            .collect_vec();
        assert_eq!(roots.len(), 1);
        let root = roots[0];
        // depth_first_search()
        let mut discovered: HashMap<NodeIndex, PatternId> = Default::default();
        let mut time = 0;
        let mut edge_count = 0;
        self.dfs_compare(&graph, root, &mut discovered, &mut time, &mut edge_count)
    }

    /// True: self is min currently
    fn dfs_compare(
        &self,
        graph: &Graph<NodeLabel, EdgeLabel>,
        u: NodeIndex,
        discovered: &mut HashMap<NodeIndex, PatternId>,
        // finished: &mut impl VisitMap<NodeId>,
        time: &mut PatternId,
        edge_count: &mut usize,
    ) -> bool {
        if let Entry::Vacant(entry) = discovered.entry(u) {
            let timestamp = *time;
            entry.insert(timestamp);
            *time += 1;
            for (_, v, el) in graph
                .edges(u)
                .map(|e| {
                    let v = e.target();
                    (
                        (
                            *discovered.get(&v).unwrap_or(time),
                            &graph[u],
                            e.weight(),
                            &graph[v],
                        ),
                        v,
                        e.weight(),
                    )
                })
                .sorted_by_key(|x| x.0)
            {
                let dfs_edge = DFSEdge {
                    i: timestamp,
                    j: *discovered.get(&v).unwrap_or(time),
                    label_i: graph[u].clone(),
                    label_ij: el.clone(),
                    label_j: graph[v].clone(),
                };
                let cnt = *edge_count;
                *edge_count += 1;
                if dfs_edge < self.edges[cnt] {
                    return false;
                }

                if !discovered.contains_key(&v) {
                    if !self.dfs_compare(graph, v, discovered, time, edge_count) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

impl fmt::Display for DFSCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, edge) in self.edges.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", edge)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Projection {
    mapping: FxHashMap<PatternId, NodeIndex>,
}

impl GSpan {
    pub fn new(
        egraph: SerializedEGraph,
        library: Library,
        min_support: usize,
    ) -> Result<Self, String> {
        let lib_pins = Self::construct_lib_pins(&library)?;
        let graph = Self::construct_graph(&egraph, &lib_pins)?;
        Ok(Self {
            egraph,
            library,
            min_support,
            lib_pins,
            graph,
            frequent_patterns: Vec::new(),
        })
    }

    fn construct_lib_pins(
        library: &Library,
    ) -> Result<IndexMap<String, (String, Vec<String>)>, String> {
        let mut lib_pins = IndexMap::default();
        for (cell_name, pins) in library {
            let out_pins = pins
                .iter()
                .filter_map(|(n, d)| match d {
                    PinDirection::O => Some(n),
                    _ => None,
                })
                .collect_vec();
            if out_pins.len() != 1 {
                return Err(format!(
                    "We only support 1 out pin now, got {} for cell {}",
                    out_pins.len(),
                    cell_name
                )
                    .to_string());
            }
            let out_pin = out_pins[0].clone();
            let in_pins = pins
                .iter()
                .filter_map(|(n, d)| match d {
                    PinDirection::I => Some(n.clone()),
                    _ => None,
                })
                .collect_vec();
            lib_pins.insert(cell_name.clone(), (out_pin, in_pins));
        }
        Ok(lib_pins)
    }

    fn construct_graph(
        egraph: &SerializedEGraph,
        lib_pins: &IndexMap<String, (String, Vec<String>)>,
    ) -> Result<Graph<NodeLabel, EdgeLabel>, String> {
        let mut graph = Graph::new();
        let class_map: IndexMap<_, _> = egraph
            .classes()
            .iter()
            .map(|(id, _)| (id.clone(), graph.add_node(NodeLabel::EClass)))
            .collect();
        let node_map: IndexMap<_, _> = egraph
            .nodes
            .iter()
            .map(|(id, n)| (id.clone(), graph.add_node(NodeLabel::ENode(n.op.clone()))))
            .collect();
        for (cid, class) in egraph.classes() {
            for nid in &class.nodes {
                let from = class_map[cid];
                let to = node_map[nid];
                let label = lib_pins
                    .get(&egraph[nid].op)
                    .map_or(String::new(), |x| x.0.clone()); // assume no label if not in lib
                // .ok_or(format!("{} is not in lib", &egraph[nid].op).to_string())?
                // .0
                // .clone();

                graph.add_edge(from, to, label); // eclass -> enode: output pin
            }
        }
        for (nid, node) in egraph.nodes.iter() {
            if lib_pins.contains_key(&node.op) {
                let num_inputs_got = node.children.len();
                let num_inputs_should = lib_pins[&node.op].1.len();
                // .get(&node.op)
                // .ok_or(format!("{} is not in lib", &egraph[nid].op).to_string())?
                // .1
                // .len();
                if num_inputs_got != num_inputs_should {
                    return Err(format!(
                        "Got {} inputs for cell {} should be {}",
                        num_inputs_got, &node.op, num_inputs_should
                    )
                        .to_string());
                }
                for (child, pin) in zip(&node.children, &lib_pins[&node.op].1) {
                    let from = node_map[nid];
                    let to = class_map[egraph.nid_to_cid(child)];
                    let label = pin.clone();
                    graph.add_edge(from, to, label); // enode -> eclass: input pin
                }
            } else {
                // assume no label if not in lib
                for child in node.children.iter() {
                    let from = node_map[nid];
                    let to = class_map[egraph.nid_to_cid(child)];
                    graph.add_edge(from, to, String::new()); // enode -> eclass: input pin
                }
            }
        }
        Ok(graph)
    }

    pub fn mine(&mut self) {
        let frequent_edges = self.find_frequent_edges();

        for edge in frequent_edges {
            let mut code = DFSCode::default();
            code.edges.push(edge);
            let projections = self.build_initial_projections(&code);
            self.subgraph_mining(&code, projections);
        }
    }

    fn find_frequent_edges(&self) -> Vec<DFSEdge> {
        let mut edge_counts = FxHashMap::default();
        self.graph
            .raw_edges()
            .iter()
            .filter(|e| self.graph[e.source()] == NodeLabel::EClass)
            .for_each(|e| {
                let dfs_edge = DFSEdge {
                    i: 0,
                    j: 1,
                    label_i: self.graph[e.source()].clone(),
                    label_ij: e.weight.clone(),
                    label_j: self.graph[e.target()].clone(),
                };
                *edge_counts.entry(dfs_edge).or_insert(0) += 1;
            });
        edge_counts
            .into_iter()
            .filter(|(_, c)| *c >= self.min_support)
            .map(|(e, _)| e)
            .collect()
    }

    fn build_initial_projections(&self, code: &DFSCode) -> Vec<Projection> {
        if code.edges.len() != 1 {
            return Vec::new();
        }
        let edge = &code.edges[0];
        self.graph
            .raw_edges()
            .iter()
            .filter_map(|graph_edge| {
                if (self.graph[graph_edge.source()] == edge.label_i)
                    && (graph_edge.weight == edge.label_ij)
                    && (self.graph[graph_edge.target()] == edge.label_j)
                {
                    Some(Projection {
                        mapping: FxHashMap::from_iter([
                            (edge.i, graph_edge.source()),
                            (edge.j, graph_edge.target()),
                        ]),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    fn subgraph_mining(&mut self, code: &DFSCode, projections: Vec<Projection>) {}
}

mod test {
    use super::*;
    use NodeLabel::{EClass, ENode};
    #[test]
    fn test_dfs_code_is_min() {
        let suppose_min_dfs_code = DFSCode {
            edges: vec![
                dfs_edge!(0, 1, EClass, "Y", ENode("XOR")),
                dfs_edge!(1, 2, ENode("XOR"), "A", EClass),
                dfs_edge!(2, 3, EClass, "Y", ENode("AND")),
                dfs_edge!(3, 4, ENode("AND"), "A", EClass),
                dfs_edge!(4, 5, EClass, "", ENode("a")),
                dfs_edge!(3, 6, ENode("AND"), "B", EClass),
                dfs_edge!(6, 7, EClass, "", ENode("b")),
                dfs_edge!(1, 8, ENode("XOR"), "B", EClass),
                dfs_edge!(8, 9, EClass, "Y", ENode("OR")),
                dfs_edge!(9, 6, ENode("OR"), "A", EClass),
                dfs_edge!(9, 10, ENode("OR"), "B", EClass),
                dfs_edge!(10, 11, EClass, "", ENode("c")),
            ]
        };
        assert!(suppose_min_dfs_code.is_min());
    }

    #[test]
    fn test_dfs_code_is_not_min() {
        let mut suppose_not_min_dfs_code = DFSCode {
            edges: vec![
                dfs_edge!(0, 1, EClass, "Y", ENode("XOR")),
                dfs_edge!(1, 2, ENode("XOR"), "A", EClass),
                dfs_edge!(2, 3, EClass, "Y", ENode("AND")),
                dfs_edge!(3, 4, ENode("AND"), "A", EClass),
                dfs_edge!(4, 5, EClass, "", ENode("a")),
                dfs_edge!(3, 6, ENode("AND"), "B", EClass),
                dfs_edge!(6, 7, EClass, "", ENode("b")),
                dfs_edge!(1, 8, ENode("XOR"), "B", EClass),
                dfs_edge!(8, 9, EClass, "Y", ENode("OR")),
                dfs_edge!(9, 10, ENode("OR"), "B", EClass),
                dfs_edge!(10, 11, EClass, "", ENode("c")),
                dfs_edge!(9, 6, ENode("OR"), "A", EClass),
            ]
        };
        assert!(!suppose_not_min_dfs_code.is_min());
        suppose_not_min_dfs_code = DFSCode {
            edges: vec![
                dfs_edge!(0, 1, EClass, "Y", ENode("XOR")),
                dfs_edge!(1, 2, ENode("XOR"), "A", EClass),
                dfs_edge!(2, 3, EClass, "Y", ENode("AND")),
                dfs_edge!(3, 4, ENode("AND"), "B", EClass),
                dfs_edge!(4, 5, EClass, "", ENode("b")),
                dfs_edge!(3, 6, ENode("AND"), "A", EClass),
                dfs_edge!(6, 7, EClass, "", ENode("a")),
                dfs_edge!(1, 8, ENode("XOR"), "B", EClass),
                dfs_edge!(8, 9, EClass, "Y", ENode("OR")),
                dfs_edge!(9, 4, ENode("OR"), "A", EClass),
                dfs_edge!(9, 10, ENode("OR"), "B", EClass),
                dfs_edge!(10, 11, EClass, "", ENode("c")),
            ]
        };
        assert!(!suppose_not_min_dfs_code.is_min());
    }


}