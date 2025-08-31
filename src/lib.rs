mod analyzer;
pub mod egraph_roots;
pub mod extractor;
pub mod io;
pub mod language;
pub mod netlist;
pub mod rule;

use crate::egraph_roots::EGraphRoots;
use crate::language::LanguageType;
use crate::netlist::Netlist;
use egg::*;
pub use egraph_serialize::EGraph as SerializedEGraph;
use extraction_gym::ExtractionResult;
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::visit::IntoNeighbors;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::Display;
use std::ops::Index;

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

pub fn netlist_to_egg_roots<N, A>(
    netlist: &Netlist<N, ()>,
) -> Result<EGraphRoots<N::Lang, A>, String>
where
    N: LanguageType,
    A: Analysis<N::Lang> + Default,
    A::Data: Clone,
{
    let mut egraph_roots: EGraphRoots<N::Lang, A> = Default::default();
    let mut nid_to_id: FxHashMap<NodeIndex, Id> = Default::default();
    let order =
        toposort(&netlist.graph, None).map_err(|e| format!("Graph contains cycle: {:?}", e))?;
    for &nid in netlist.leaves.iter() {
        let weight = &netlist.graph[nid];
        let id = egraph_roots.egraph.add(weight.to_lang_input());
        nid_to_id.insert(nid, id);
    }
    let leaves_set = FxHashSet::from_iter(netlist.leaves.clone());
    let root_set = FxHashSet::from_iter(netlist.roots.clone());
    for &nid in order.iter().rev() {
        if leaves_set.contains(&nid) || root_set.contains(&nid) {
            continue;
        }
        if nid_to_id.contains_key(&nid) {
            return Err(format!("Node {:?} exists already", nid));
        } else {
            let inputs: Vec<_> = netlist
                .graph
                .neighbors(nid)
                .map(|neighbor| nid_to_id[&neighbor])
                .collect();
            let inputs: Vec<_> = inputs.into_iter().rev().collect(); // petaGraph use linked list to push edges, so we must reverse
            let weight = &netlist.graph[nid];
            let id = egraph_roots.egraph.add(weight.to_lang_gate(inputs));
            nid_to_id.insert(nid, id);
        }
    }
    for &nid in netlist.roots.iter() {
        if nid_to_id.contains_key(&nid) {
            return Err(format!("Node {:?} exists already", nid));
        } else {
            let count = netlist.graph.neighbors(nid).count();
            if count != 1 {
                return Err(format!(
                    "Output {:?} should have exactly 1 input, but got {:?}",
                    nid, count
                ));
            }
            let inputs: Vec<_> = netlist
                .graph
                .neighbors(nid)
                .map(|neighbor| nid_to_id[&neighbor])
                .collect();
            let weight = &netlist.graph[nid];
            let id = egraph_roots.egraph.add(weight.to_lang_output(inputs[0]));
            nid_to_id.insert(nid, id);
            egraph_roots.roots.push(id);
        }
    }
    egraph_roots.egraph.rebuild();
    Ok(egraph_roots)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::liberty::{get_direction_of_pins, read_liberty};
    use crate::io::stdcell::{read_bench_to_netlist, read_verilog_with_lib_to_netlist};
    use std::env;

    #[test]
    fn test_netlist_to_egg_roots() {
        let netlist = read_bench_to_netlist("test/add2.bench").unwrap();
        let egraph_roots: EGraphRoots<_, ()> = netlist_to_egg_roots(&netlist).unwrap();
        let s = SerializedEGraph::from(&egraph_roots);
        s.to_json_file(
            env::current_dir()
                .unwrap()
                .join("json/test_add2_bench.json"),
        )
        .unwrap();
        #[cfg(target_os = "linux")]
        s.to_svg_file(env::current_dir().unwrap().join("svg/test_add2_bench.svg"))
            .unwrap();
        let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
        let lib = get_direction_of_pins(&liberty).unwrap();
        let (netlist, name) = read_verilog_with_lib_to_netlist("test/add2_map_abc.v", lib).unwrap();
        assert_eq!(name, "add2");
        let egraph_roots: EGraphRoots<_, ()> = netlist_to_egg_roots(&netlist).unwrap();
        let s = SerializedEGraph::from(&egraph_roots);
        s.to_json_file(
            env::current_dir()
                .unwrap()
                .join("json/test_add2_map_abc_v.json"),
        )
        .unwrap();
        #[cfg(target_os = "linux")]
        s.to_svg_file(
            env::current_dir()
                .unwrap()
                .join("svg/test_add2_map_abc_v.svg"),
        )
        .unwrap();
        let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
        let lib = get_direction_of_pins(&liberty).unwrap();
        let (netlist, name) = read_verilog_with_lib_to_netlist("test/mul4_map_abc.v", lib).unwrap();
        assert_eq!(name, "Multi4");
        let egraph_roots: EGraphRoots<_, ()> = netlist_to_egg_roots(&netlist).unwrap();
        let s = SerializedEGraph::from(&egraph_roots);
        s.to_json_file(
            env::current_dir()
                .unwrap()
                .join("json/test_mul4_map_abc_v.json"),
        )
        .unwrap();
        #[cfg(target_os = "linux")]
        s.to_svg_file(
            env::current_dir()
                .unwrap()
                .join("svg/test_mul4_map_abc_v.svg"),
        )
        .unwrap();
    }

    #[test]
    fn test_netlist_to_egg_roots_mul32() {
        let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
        let lib = get_direction_of_pins(&liberty).unwrap();
        let (netlist, name) =
            read_verilog_with_lib_to_netlist("../test/mul32_map_genus.v", lib).unwrap();
        assert_eq!(name, "Multiplier");
        let egraph_roots: EGraphRoots<_, ()> = netlist_to_egg_roots(&netlist).unwrap();
        let s = SerializedEGraph::from(&egraph_roots);
        s.to_json_file(
            env::current_dir()
                .unwrap()
                .join("json/test_mul32_map_genus_v.json"),
        )
        .unwrap();
        s.to_dot_file(
            env::current_dir()
                .unwrap()
                .join("dot/test_mul32_map_genus_v_egg.dot"),
        )
        .unwrap()

        // dot rendering is too slow, so commented
        // #[cfg(target_os = "linux")]
        // s.to_svg_file(
        //     env::current_dir()
        //         .unwrap()
        //         .join("svg/test_mul4_map_genus_v.svg"),
        // )
        // .unwrap();
    }
}
