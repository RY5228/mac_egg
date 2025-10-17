use crate::SerializedEGraph;
use crate::io::liberty::Library;
use derivative::Derivative;
use egraph_serialize::ClassId;
use indexmap::{IndexMap, IndexSet};
use itertools::{Itertools, sorted};
use libertyparse::PinDirection;
use petgraph::acyclic::Acyclic;
use petgraph::algo::toposort;
use petgraph::graph::{Edge, EdgeReference, Edges, NodeIndex};
use petgraph::prelude::{EdgeIndex, EdgeRef};
use petgraph::visit::{
    EdgeCount, IntoEdges, IntoNeighbors, NodeRef, VisitMap, Visitable, depth_first_search,
};
use petgraph::{Directed, Graph, Incoming};
use rustc_hash::{FxHashMap, FxHashSet};
use std::cmp::Reverse;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::iter::{Filter, zip};
use std::ops::{ControlFlow, Index};

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
    frequent_patterns: Vec<(DFSCode, usize)>,
    max_size: usize,
    max_num_inputs: usize,
    finished_edges: IndexSet<EdgeIndex>,
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

pub enum DFSCodeConstraint {
    NotYet,
    Satisfied,
    Violate,
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

    pub fn to_graph_pid(&self) -> Graph<(NodeLabel, PatternId), EdgeLabel> {
        use std::collections::hash_map::Entry;
        let mut graph = Graph::new();
        let mut node_map = FxHashMap::default();
        for edge in &self.edges {
            let from = match node_map.entry(edge.i) {
                Entry::Vacant(e) => {
                    let nid = graph.add_node((edge.label_i.clone(), edge.i));
                    e.insert(nid);
                    nid
                }
                Entry::Occupied(e) => *e.get(),
            };
            let to = match node_map.entry(edge.j) {
                Entry::Vacant(e) => {
                    let nid = graph.add_node((edge.label_j.clone(), edge.j));
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

    pub fn is_min(&self, graph_cache: Option<&Graph<NodeLabel, EdgeLabel>>) -> bool {
        if self.edges.len() == 1 {
            return true;
        }
        let graph = if let Some(g) = graph_cache {
            g
        } else {
            &self.to_graph()
        };
        // ensure DAG
        if toposort(&graph, None).is_err() {
            return false;
        }
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

    pub fn satisfy_constraints(
        &self,
        graph_cache: Option<&Graph<NodeLabel, EdgeLabel>>,
        max_size: usize,
        max_num_inputs: usize,
        lib_pins: &IndexMap<String, (String, Vec<String>)>,
    ) -> DFSCodeConstraint {
        if self.edges.len() == 1 {
            // println!("Only one edge");
            return DFSCodeConstraint::NotYet;
        }
        let graph = if let Some(g) = graph_cache {
            g
        } else {
            &self.to_graph()
        };

        let enode_count = graph
            .node_weights()
            .filter(|w| {
                if let NodeLabel::ENode(_) = w {
                    true
                } else {
                    false
                }
            })
            .count();

        let mut not_yet = false;
        if enode_count > max_size {
            return DFSCodeConstraint::Violate;
        } else if enode_count < 2 {
            // println!("No enough enodes");
            not_yet = true;
        }

        // ensure DAG
        for n in toposort(&graph, None).unwrap() {
            match &graph[n] {
                NodeLabel::EClass => {
                    if graph.neighbors(n).count() > 1 {
                        return DFSCodeConstraint::Violate;
                    }
                }
                NodeLabel::ENode(op) => {
                    let num_neighbors = graph.neighbors(n).count();
                    if num_neighbors == 0 {
                        // println!("ENode as leaf");
                        not_yet = true;
                    } else {
                        if let Some((_, pins)) = lib_pins.get(op) {
                            if num_neighbors > pins.len() {
                                return DFSCodeConstraint::Violate;
                            } else if num_neighbors < pins.len() {
                                // println!("ENode no enough input");
                                not_yet = true;
                            }
                        }
                    }
                }
            }
        }
        if not_yet {
            DFSCodeConstraint::NotYet
        } else if graph
            .node_indices()
            .filter(|&i| graph.neighbors(i).count() == 0)
            .count()
            > max_num_inputs
        {
            // println!("Too many inputs");
            DFSCodeConstraint::NotYet
        } else {
            DFSCodeConstraint::Satisfied
        }
    }

    pub fn node_indices(&self) -> IndexSet<PatternId> {
        let mut node_indices = IndexSet::default();
        for e in &self.edges {
            node_indices.insert(e.i);
            node_indices.insert(e.j);
        }
        node_indices
    }

    pub fn edge_indices(&self) -> IndexMap<PatternId, IndexSet<PatternId>> {
        let mut edge_indices: IndexMap<PatternId, IndexSet<PatternId>> = Default::default();
        for e in &self.edges {
            edge_indices.entry(e.i).or_default().insert(e.j);
            // edge_indices.insert((e.i, e.j));
        }
        edge_indices
    }

    pub fn to_acyclic(&self) -> Option<Acyclic<Graph<NodeLabel, EdgeLabel>>> {
        Acyclic::try_from(self.to_graph()).ok()
    }

    pub fn to_acyclic_pid(&self) -> Option<Acyclic<Graph<(NodeLabel, PatternId), EdgeLabel>>> {
        Acyclic::try_from(self.to_graph_pid()).ok()
    }

    pub fn try_acyclic(
        graph: &Graph<NodeLabel, EdgeLabel>,
    ) -> Option<Acyclic<Graph<NodeLabel, EdgeLabel>>> {
        Acyclic::try_from(graph.clone()).ok()
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
    reverse: FxHashMap<NodeIndex, PatternId>,
}

impl FromIterator<(PatternId, NodeIndex)> for Projection {
    fn from_iter<T: IntoIterator<Item = (PatternId, NodeIndex)>>(iter: T) -> Projection {
        let mut mapping = FxHashMap::default();
        let mut reverse = FxHashMap::default();
        for (k, v) in iter {
            mapping.insert(k, v);
            reverse.insert(v, k);
        }
        Projection { mapping, reverse }
    }
}

impl Index<PatternId> for Projection {
    type Output = NodeIndex;
    fn index(&self, i: PatternId) -> &Self::Output {
        &self.mapping[&i]
    }
}

impl Index<NodeIndex> for Projection {
    type Output = PatternId;
    fn index(&self, i: NodeIndex) -> &Self::Output {
        &self.reverse[&i]
    }
}

impl Projection {
    fn insert(&mut self, pi: PatternId, gi: NodeIndex) -> (Option<PatternId>, Option<NodeIndex>) {
        let old_gi = self.mapping.insert(pi, gi);
        let old_pi = self.reverse.insert(gi, pi);
        (old_pi, old_gi)
    }

    fn get_gi(&self, pi: &PatternId) -> Option<&NodeIndex> {
        self.mapping.get(pi)
    }

    fn get_pi(&self, gi: &NodeIndex) -> Option<&PatternId> {
        self.reverse.get(gi)
    }
}

impl GSpan {
    pub fn new(
        egraph: SerializedEGraph,
        library: Library,
        min_support: usize,
        max_size: usize,
        max_num_inputs: usize,
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
            max_size,
            max_num_inputs,
            finished_edges: Default::default(),
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
            code.edges.push(edge.clone());
            let projections = self.build_initial_projections(&edge);
            self.subgraph_mining(&code, projections);
            self.finish_edge(&edge);
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
            .sorted()
            .collect()
    }

    fn build_initial_projections(&self, edge: &DFSEdge) -> Vec<Projection> {
        self.graph
            .raw_edges()
            .iter()
            .filter_map(|graph_edge| {
                if (self.graph[graph_edge.source()] == edge.label_i)
                    && (graph_edge.weight == edge.label_ij)
                    && (self.graph[graph_edge.target()] == edge.label_j)
                {
                    Some(Projection::from_iter([
                        (edge.i, graph_edge.source()),
                        (edge.j, graph_edge.target()),
                    ]))
                } else {
                    None
                }
            })
            .collect()
    }

    fn subgraph_mining(&mut self, code: &DFSCode, projections: Vec<Projection>) {
        // println!("Mining: support: {}, code: {:#?} ", projections.len(), code);
        let code_graph = code.to_graph();
        let graph_cache = Some(&code_graph);
        if !code.is_min(graph_cache) {
            // println!("Not minimum");
            return;
        }
        match code.satisfy_constraints(
            graph_cache,
            self.max_size,
            self.max_num_inputs,
            &self.lib_pins,
        ) {
            DFSCodeConstraint::Violate => {
                // println!("Violate");
                return;
            }
            DFSCodeConstraint::Satisfied => {
                self.frequent_patterns
                    .push((code.clone(), projections.len()));
                // println!("Satisfied");
            }
            DFSCodeConstraint::NotYet => {
                // println!("NotYet");
            }
        }

        let extensions = self.find_all_possible_extensions(code, &projections);

        for (new_code, new_projections) in extensions {
            self.subgraph_mining(&new_code, new_projections);
        }
    }

    fn find_all_possible_extensions(
        &self,
        code: &DFSCode,
        projections: &[Projection],
        // graph_pid_cache: Option<&Acyclic<Graph<(NodeLabel, PatternId), EdgeLabel>>>
    ) -> Vec<(DFSCode, Vec<Projection>)> {
        let code_nodes = code.node_indices();
        let code_edges = code.edge_indices();

        let contains_edge = |i, j| code_edges.get(i).map(|n| n.contains(j)).unwrap_or(false);
        // let acyclic_pid = if let Some(g) = graph_pid_cache {
        //     g
        // } else {
        //     let acyclic =  code.to_acyclic_pid();
        //     if acyclic.is_none() {
        //         return Vec::new();
        //     }
        //     &acyclic.unwrap()
        // };
        // acyclic_pid.raw_nodes().iter().filter_map(|n|{
        //     let (label, id) = n.weight;
        //
        //     todo!()
        // })
        let mut extensions: IndexMap<DFSCode, Vec<Projection>> = Default::default();
        for projection in projections {
            for node in code_nodes.iter() {
                let graph_node = projection[*node];
                if let NodeLabel::ENode(op) = &self.graph[graph_node] {
                    if self.lib_pins.get(op).is_some() {
                        let min_edge = self
                            .graph
                            .edges(graph_node)
                            .filter_map(|graph_edge| {
                                if self.finished_edges.contains(&graph_edge.id()) {
                                    return None;
                                }
                                let target = graph_edge.target();
                                if let Some(pi) = projection.get_pi(&target) {
                                    if contains_edge(node, pi) {
                                        None
                                    } else {
                                        let new_edge = DFSEdge {
                                            i: *node,
                                            j: *pi,
                                            label_i: self.graph[graph_node].clone(),
                                            label_ij: graph_edge.weight().clone(),
                                            label_j: self.graph[target].clone(),
                                        };
                                        let mut new_code = code.clone();
                                        new_code.edges.push(new_edge.clone());
                                        if new_code.to_acyclic().is_none() {
                                            None
                                        } else {
                                            Some((new_edge, target, new_code))
                                        }
                                    }
                                } else {
                                    let new_edge = DFSEdge {
                                        i: *node,
                                        j: code_nodes.len(),
                                        label_i: self.graph[graph_node].clone(),
                                        label_ij: graph_edge.weight().clone(),
                                        label_j: self.graph[target].clone(),
                                    };
                                    let mut new_code = code.clone();
                                    new_code.edges.push(new_edge.clone());
                                    Some((new_edge, target, new_code))
                                }
                            })
                            .min_by_key(|x| x.0.clone());
                        if let Some((min_edge, target, new_code)) = min_edge {
                            let mut new_projection = projection.clone();
                            new_projection.insert(min_edge.j, target);
                            extensions.entry(new_code).or_default().push(new_projection);
                        }
                    }
                }
            }
        }
        // if extensions.len() == 0 {
        for projection in projections {
            for node in code_nodes.iter() {
                if code_edges.contains_key(node) {
                    continue;
                }
                let graph_node = projection[*node];
                if let NodeLabel::EClass = &self.graph[graph_node] {
                    let min_edge = self
                        .graph
                        .edges(graph_node)
                        .filter_map(|graph_edge| {
                            if self.finished_edges.contains(&graph_edge.id()) {
                                return None;
                            }
                            let target = graph_edge.target();
                            if projection.get_pi(&target).is_some() {
                                panic!("Two EClasses point to the same ENode!")
                            }
                            let new_edge = DFSEdge {
                                i: *node,
                                j: code_nodes.len(),
                                label_i: self.graph[graph_node].clone(),
                                label_ij: graph_edge.weight().clone(),
                                label_j: self.graph[target].clone(),
                            };
                            let mut new_code = code.clone();
                            new_code.edges.push(new_edge.clone());
                            Some((new_edge, target, new_code))
                        })
                        .min_by_key(|x| x.0.clone());
                    if let Some((min_edge, target, new_code)) = min_edge {
                        let mut new_projection = projection.clone();
                        new_projection.insert(min_edge.j, target);
                        extensions.entry(new_code).or_default().push(new_projection);
                    }
                }
            }
        }
        // }
        extensions
            .into_iter()
            .filter(|(_, projections)| projections.len() >= self.min_support)
            .collect()
    }

    fn finish_edge(&mut self, edge: &DFSEdge) {
        self.finished_edges
            .extend(self.graph.edge_references().filter_map(|graph_edge| {
                if (self.graph[graph_edge.source()] == edge.label_i)
                    && (graph_edge.weight() == &edge.label_ij)
                    && (self.graph[graph_edge.target()] == edge.label_j)
                {
                    Some(graph_edge.id())
                } else {
                    None
                }
            }));
    }
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
            ],
        };
        assert!(suppose_min_dfs_code.is_min(None));
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
            ],
        };
        assert!(!suppose_not_min_dfs_code.is_min(None));
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
            ],
        };
        assert!(!suppose_not_min_dfs_code.is_min(None));
    }

    #[test]
    fn test_add2() {
        use crate::choose_result_in_serialized_egraph_into_netlist;
        use crate::egraph_roots::EGraphRoots;
        use crate::io::liberty::{get_direction_of_pins, read_liberty};
        use crate::io::stdcell::{
            read_verilog_with_lib_to_netlist, write_verilog_from_netlist_with_lib,
        };
        use crate::language::StdCellLanguage;
        use crate::language::StdCellType;
        use crate::rule::JsonRules;
        use crate::{egg_to_serialized_egraph, netlist_to_egg_roots};
        use egg::Runner;
        use petgraph::dot::{Config, Dot};
        use std::env;
        use std::fs;

        let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
        let lib = get_direction_of_pins(&liberty).unwrap();
        let (netlist, name) =
            read_verilog_with_lib_to_netlist("test/add2_map_abc.v", lib.clone()).unwrap();
        assert_eq!(name, "add2");
        let egraph_roots: EGraphRoots<_, ()> = netlist_to_egg_roots(&netlist).unwrap();
        let mut rules =
            JsonRules::from_path(env::current_dir().unwrap().join("test/6t_inv_rules.json"))
                .unwrap()
                .into_egg_rules::<StdCellLanguage>()
                .unwrap();
        rules.extend(
            JsonRules::from_path(env::current_dir().unwrap().join("test/6t_dmg_rules.json"))
                .unwrap()
                .into_egg_rules::<StdCellLanguage>()
                .unwrap(),
        );
        let runner = Runner::default()
            .with_egraph(egraph_roots.egraph)
            .run(&rules);
        let s = egg_to_serialized_egraph(&runner.egraph, &egraph_roots.roots);
        s.to_json_file(
            env::current_dir()
                .unwrap()
                .join("json/test_add2_map_abc_inv_dmg_rules.json"),
        )
        .unwrap();
        let mut gspan = GSpan::new(s, lib, 2, 5, 3).unwrap();
        // let mut graph = gspan.graph.clone();
        // let sources = graph
        //     .node_indices()
        //     .filter(|n| graph.edges_directed(*n, Incoming).count() == 0)
        //     .collect_vec();
        // let sinks = graph
        //     .node_indices()
        //     .filter(|n| graph.edges(*n).count() == 0)
        //     .collect_vec();
        // let global_source = graph.add_node(ENode("_global_source".to_owned()));
        // let global_sink = graph.add_node(ENode("_global_sink".to_owned()));
        // for n in sources {
        //     graph.add_edge(global_source, n, "_global_source".to_owned());
        // }
        // for n in sinks {
        //     graph.add_edge(n, global_sink, "_global_sink".to_owned());
        // }
        // let fancy_dot = Dot::with_attr_getters(
        //     &graph,
        //     &[],
        //     &|_, e| {
        //         if e.weight() == "_global_source" || e.weight() == "_global_sink" {
        //             "style=invis".to_owned()
        //         } else {
        //             String::new()
        //         }
        //     },
        //     &|g, (n, l)| {
        //         if let ENode(s) = l {
        //             if s == "_global_source" || s == "_global_sink" {
        //                 "style=invis".to_owned()
        //             } else {
        //                 String::new()
        //             }
        //         } else {
        //             String::new()
        //         }
        //     },
        // );
        let fancy_dot = Dot::new(&gspan.graph);
        fs::write(
            env::current_dir()
                .unwrap()
                .join("dot/test_add2_map_abc_inv_dmg_rules.dot"),
            format!("{:?}", fancy_dot),
        )
        .unwrap();
        let frequent_edges = gspan.find_frequent_edges();
        println!("{:#?}", frequent_edges);
        let projections = gspan.build_initial_projections(&frequent_edges[0]);
        println!("{:#?}", projections);
        gspan.mine();
        println!("patterns = {:#?}", gspan.frequent_patterns);
        println!("{}", gspan.frequent_patterns.len());
    }

    #[test]
    fn test_mul32() {
        use crate::choose_result_in_serialized_egraph_into_netlist;
        use crate::egraph_roots::EGraphRoots;
        use crate::io::liberty::{get_direction_of_pins, read_liberty};
        use crate::io::stdcell::{
            read_verilog_with_lib_to_netlist, write_verilog_from_netlist_with_lib,
        };
        use crate::language::StdCellLanguage;
        use crate::language::StdCellType;
        use crate::rule::JsonRules;
        use crate::{egg_to_serialized_egraph, netlist_to_egg_roots};
        use egg::Runner;
        use petgraph::dot::{Config, Dot};
        use std::env;
        use std::fs;

        let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
        let lib = get_direction_of_pins(&liberty).unwrap();
        let (netlist, name) =
            read_verilog_with_lib_to_netlist("test/mul32_map_genus.v", lib.clone()).unwrap();
        assert_eq!(name, "Multiplier");
        let egraph_roots: EGraphRoots<_, ()> = netlist_to_egg_roots(&netlist).unwrap();
        let mut rules =
            JsonRules::from_path(env::current_dir().unwrap().join("test/6t_inv_rules.json"))
                .unwrap()
                .into_egg_rules::<StdCellLanguage>()
                .unwrap();
        rules.extend(
            JsonRules::from_path(env::current_dir().unwrap().join("test/6t_dmg_rules.json"))
                .unwrap()
                .into_egg_rules::<StdCellLanguage>()
                .unwrap(),
        );
        println!("{}", egraph_roots.egraph.total_number_of_nodes());
        let runner = Runner::default()
            .with_egraph(egraph_roots.egraph)
            .with_node_limit(30000)
            .run(&rules);
        println!("{:?}", runner.stop_reason);
        println!("{}", runner.egraph.total_number_of_nodes());
        // return;
        let s = egg_to_serialized_egraph(&runner.egraph, &egraph_roots.roots);
        s.to_json_file(
            env::current_dir()
                .unwrap()
                .join("json/test_mul32_map_genus_inv_dmg_rules.json"),
        )
        .unwrap();
        let mut gspan = GSpan::new(s, lib, 10, 5, 3).unwrap();

        let fancy_dot = Dot::new(&gspan.graph);
        fs::write(
            env::current_dir()
                .unwrap()
                .join("dot/test_mul32_map_genus_inv_dmg_rules.dot"),
            format!("{:?}", fancy_dot),
        )
        .unwrap();
        let frequent_edges = gspan.find_frequent_edges();
        println!("{}", frequent_edges.len());
        let projections = gspan.build_initial_projections(&frequent_edges[0]);
        println!("{}", projections.len());
        gspan.mine();
        // println!("patterns = {:#?}", gspan.frequent_patterns);
        println!("{}", gspan.frequent_patterns.len());
        fs::write(
            env::current_dir()
                .unwrap()
                .join("log/test_mul32_map_genus_inv_dmg_rules.log"),
            format!("{:#?}", gspan.frequent_patterns),
        )
        .unwrap();
    }
}
