use crate::io::bench::parse_bench;
use crate::language::StdCellType;
use crate::netlist::Netlist;
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::fs;
use std::path::Path;

pub fn read_bench_to_netlist<P: AsRef<Path>>(path: P) -> Result<Netlist<StdCellType, ()>, String> {
    let content =
        fs::read_to_string(path.as_ref()).map_err(|e| format!("Error reading file: {}", e))?;
    let (_, bench_file) = parse_bench(&content).map_err(|e| format!("Parse error: {:?}", e))?;
    let mut netlist: Netlist<StdCellType, ()> = Default::default();
    let mut symbol_to_nid: FxHashMap<&str, NodeIndex> = Default::default();

    fn handle_symbol(symbol: &str) -> StdCellType {
        match symbol {
            "0" => StdCellType::Bool(false),
            "1" => StdCellType::Bool(true),
            s => StdCellType::Symbol(s.into()),
        }
    }
    for input in bench_file.inputs {
        if let Entry::Vacant(entry) = symbol_to_nid.entry(input) {
            let nid = netlist.graph.add_node(handle_symbol(input));
            entry.insert(nid);
            netlist.leaves.push(nid);
        } else {
            return Err(format!("input {} exists already", input).into());
        }
    }
    for gate in bench_file.gates {
        let oid = *symbol_to_nid
            .entry(gate.output)
            .or_insert_with(|| netlist.graph.add_node(handle_symbol(gate.gate_type)));
        for input in gate.inputs {
            let iid = *symbol_to_nid
                .entry(input)
                .or_insert_with(|| netlist.graph.add_node(handle_symbol(input)));
            netlist.graph.add_edge(oid, iid, ());
        }
    }
    for output in bench_file.outputs {
        let oid = netlist.graph.add_node(handle_symbol(output));
        let iid = *symbol_to_nid
            .get(output)
            .ok_or(format!("output {} not found", output))?;
        netlist.graph.add_edge(oid, iid, ());
        netlist.roots.push(oid);
    }
    Ok(netlist)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::dot::{Config, Dot};
    use std::env;

    #[test]
    fn test_read_add2_bench_netlist() {
        let netlist = read_bench_to_netlist("test/add2.bench").unwrap();
        fs::write(
            env::current_dir().unwrap().join("dot/test_add2_bench.dot"),
            format!(
                "{:?}",
                Dot::with_config(&netlist.graph, &[Config::EdgeNoLabel])
            ),
        )
        .unwrap();
    }
}
