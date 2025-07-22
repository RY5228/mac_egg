mod egraph_roots;
pub mod extractor;
pub mod io;
pub mod language;
mod netlist;
pub mod rule;

use crate::egraph_roots::EGraphRoots;
use crate::language::LanguageType;
use crate::netlist::Netlist;
use egg::*;
pub use egraph_serialize::EGraph as SerializedEGraph;
use extraction_gym::ExtractionResult;
use rustc_hash::FxHashSet;
use std::fmt::Display;

pub fn egg_to_serialized_egraph<L, N>(egraph: &EGraph<L, N>, roots: &Vec<Id>) -> SerializedEGraph
where
    L: Language + Display,
    N: Analysis<L>,
{
    use egraph_serialize::*;
    let mut out = EGraph::default();
    for class in egraph.classes() {
        for (i, node) in class.nodes.iter().enumerate() {
            out.add_node(
                format!("{}.{}", class.id, i),
                Node {
                    op: node.to_string(),
                    children: node
                        .children()
                        .iter()
                        .map(|id| NodeId::from(format!("{}.0", id)))
                        .collect(),
                    eclass: ClassId::from(format!("{}", class.id)),
                    cost: Cost::new(1.0).unwrap(),
                },
            )
        }
    }
    for root in roots {
        let id = egraph[*root].id;
        out.root_eclasses.push(ClassId::from(format!("{}", id)));
    }
    out
}

pub fn netlist_to_egg_roots<N, A>(netlist: &Netlist<N, ()>) -> EGraphRoots<N::Lang, A>
where
    N: LanguageType,
    A: Analysis<N::Lang> + Default,
    A::Data: Clone,
{
    todo!()
}

pub fn choose_result_in_serialized_egraph_into_netlist<N>(
    in_egraph: &SerializedEGraph,
    result: &ExtractionResult,
) -> Option<Netlist<N, ()>>
where
    N: LanguageType,
{
    use egraph_serialize::*;
    let mut netlist: Netlist<N, ()> = Default::default();
    let mut todo: Vec<ClassId> = in_egraph.root_eclasses.to_vec();
    let mut visited: FxHashSet<ClassId> = Default::default();
    while let Some(cid) = todo.pop() {
        if !visited.insert(cid.clone()) {
            continue;
        }
        assert!(result.choices.contains_key(&cid));
        let nid = &result.choices[&cid];
        netlist.graph.add_node(N::from_op(&in_egraph[nid].op));
        for child in in_egraph[nid].children.iter() {
            todo!();
        }

        for child in &in_egraph[&result.choices[&cid]].children {
            todo.push(in_egraph.nid_to_cid(child).clone());
        }
    }
    Some(netlist)
}

pub fn serialized_egraph_to_egg<L, A>(egraph: &SerializedEGraph) -> (EGraph<L, A>, Vec<Id>)
where
    L: Language + Display,
    A: Analysis<L>,
{
    todo!()
}

pub fn choose_result_in_egraph(
    in_egraph: &SerializedEGraph,
    result: &ExtractionResult,
) -> Option<SerializedEGraph> {
    use egraph_serialize::*;
    let mut out_egraph = EGraph::default();
    let mut todo: Vec<ClassId> = in_egraph.root_eclasses.to_vec();
    let mut visited: FxHashSet<ClassId> = Default::default();
    while let Some(cid) = todo.pop() {
        if !visited.insert(cid.clone()) {
            continue;
        }
        assert!(result.choices.contains_key(&cid));
        let nid = &result.choices[&cid];
        out_egraph.add_node(
            nid.to_string(),
            Node {
                op: in_egraph[nid].op.clone(),
                children: in_egraph[nid]
                    .children
                    .iter()
                    .map(|nid| result.choices[in_egraph.nid_to_cid(nid)].clone())
                    .collect(),
                eclass: in_egraph[nid].eclass.clone(),
                cost: in_egraph[nid].cost,
            },
        );

        for child in &in_egraph[&result.choices[&cid]].children {
            todo.push(in_egraph.nid_to_cid(child).clone());
        }
    }
    Some(out_egraph)
}
