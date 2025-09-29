use crate::SerializedEGraph;
use crate::io::liberty::Library;
use egraph_serialize::ClassId;
use indexmap::IndexMap;
use itertools::Itertools;
use libertyparse::PinDirection;
use petgraph::Graph;
use petgraph::graph::NodeIndex;
use petgraph::visit::NodeRef;
use rustc_hash::FxHashMap;
use std::fmt;
use std::iter::zip;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeLabel {
    Class,
    Node(String),
}

pub type EdgeLabel = String;

impl fmt::Display for NodeLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Class => write!(f, "Class"),
            Self::Node(name) => write!(f, "Node_{}", name),
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
//         src_id: PatternId,
//         src_label: &str,
//         src_pin: &str,
//         dst_id: PatternId,
//         dst_label: &str,
//         dst_pin: &str,
//     ) -> Self {
//         Self {
//             i: src_id,
//             j: dst_id,
//             label_i: src_label.into(),
//             label_ij: (src_pin.into(), dst_pin.into()),
//             label_j: dst_label.into(),
//         }
//     }
// }

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
            .map(|(id, _)| (id.clone(), graph.add_node(NodeLabel::Class)))
            .collect();
        let node_map: IndexMap<_, _> = egraph
            .nodes
            .iter()
            .map(|(id, n)| (id.clone(), graph.add_node(NodeLabel::Node(n.op.clone()))))
            .collect();
        for (cid, class) in egraph.classes() {
            for nid in &class.nodes {
                let from = class_map[cid];
                let to = node_map[nid];
                let label = lib_pins
                    .get(&egraph[nid].op)
                    .ok_or(format!("{} is not in lib", &egraph[nid].op).to_string())?
                    .0
                    .clone();
                graph.add_edge(from, to, label);
            }
        }
        for (nid, node) in egraph.nodes.iter() {
            let num_inputs_got = node.children.len();
            let num_inputs_should = lib_pins
                .get(&node.op)
                .ok_or(format!("{} is not in lib", &egraph[nid].op).to_string())?
                .1
                .len();
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
                graph.add_edge(from, to, label);
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
            .filter(|e| self.graph[e.source()] == NodeLabel::Class)
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

    fn subgraph_mining(&mut self, code: &DFSCode, projections: Vec<Projection>) {

    }
}
