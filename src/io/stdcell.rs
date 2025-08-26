use crate::io::bench::parse_bench;
use crate::io::verilog::module;
use crate::language::StdCellType;
use crate::netlist::Netlist;
use libertyparse::PinDirection;
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
            return Err(format!("input {:?} exists already", input).into());
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

pub fn read_verilog_with_lib_to_netlist<P: AsRef<Path>>(
    verilog_path: P,
    lib: Vec<(String, Vec<(String, &PinDirection)>)>,
) -> Result<(Netlist<StdCellType, ()>, String), String> {
    let content = fs::read_to_string(verilog_path.as_ref())
        .map_err(|e| format!("Error reading file: {}", e))?;
    let (_, parsed_module) = module(&content).map_err(|e| format!("Parse error: {:?}", e))?;
    parsed_module.verify()?;
    let mut netlist: Netlist<StdCellType, ()> = Default::default();
    let mut symbol_to_nid: FxHashMap<String, NodeIndex> = Default::default();

    for input in parsed_module.inputs {
        if let Some((low, high)) = input.bit_range {
            (low..=high).for_each(|bit| {
                let symbol = format!("{}[{}]", input.name, bit);
                let nid = netlist
                    .graph
                    .add_node(StdCellType::Symbol((&symbol).into()));
                symbol_to_nid.insert(symbol, nid);
                netlist.leaves.push(nid);
            });
        } else {
            let nid = netlist
                .graph
                .add_node(StdCellType::Symbol(input.name.into()));
            symbol_to_nid.insert(input.name.into(), nid);
            netlist.leaves.push(nid);
        }
    }

    let lib = lib
        .into_iter()
        .map(|(name, pins)| (name, pins.into_iter().collect::<FxHashMap<_, _>>()))
        .collect::<FxHashMap<_, _>>();
    for gate in parsed_module.gates {
        if let Some(pins) = lib.get(gate.gate_type) {
            let mut oids: Vec<_> = Default::default();
            let mut iids: Vec<_> = Default::default();
            for connection in gate.connections {
                match pins.get(connection.gate_pin) {
                    Some(PinDirection::O) => {
                        let symbol = format!("{}", connection.bit);
                        oids.push(*symbol_to_nid.entry(symbol).or_insert_with(|| {
                            netlist
                                .graph
                                .add_node(StdCellType::Symbol(gate.gate_type.into()))
                        }));
                    }
                    Some(PinDirection::I) => {
                        let symbol = format!("{}", connection.bit);
                        iids.push(*symbol_to_nid.entry(symbol).or_insert_with(|| {
                            netlist
                                .graph
                                .add_node(StdCellType::Symbol(gate.gate_type.into()))
                        }));
                    }
                    Some(PinDirection::Unsupported(s)) => {
                        return Err(format!(
                            "gate {} has pin {} with unsupported direction {}",
                            gate.gate_type, connection.gate_pin, s
                        )
                        .into());
                    }
                    Some(PinDirection::Unspecified) => {
                        return Err(format!(
                            "gate {} has pin {} with unspecified direction",
                            gate.gate_type, connection.gate_pin
                        )
                        .into());
                    }
                    None => {
                        return Err(format!(
                            "pin {} not found in gate {}",
                            connection.gate_pin, gate.gate_type
                        )
                        .into());
                    }
                }
            }
            if oids.len() != 1 {
                return Err(format!(
                    "Now only support one output gate, got {} outputs for gate {}",
                    oids.len(),
                    gate.gate_type
                )
                .into());
            }
            //     todo!(consider check the pin usage of gate)
            let oid = oids[0];
            for iid in iids {
                netlist.graph.add_edge(oid, iid, ());
            }
        } else {
            return Err(format!("gate {} not found in lib", gate.gate_type).into());
        }
    }

    for output in parsed_module.outputs {
        if let Some((low, high)) = output.bit_range {
            for bit in low..=high {
                let symbol = format!("{}[{}]", output.name, bit);
                let oid = netlist
                    .graph
                    .add_node(StdCellType::Symbol((&symbol).into()));
                let iid = *symbol_to_nid
                    .get(&symbol)
                    .ok_or(format!("output {} not found", symbol))?;
                netlist.graph.add_edge(oid, iid, ());
                netlist.roots.push(oid);
            }
        } else {
            let symbol = output.name;
            let oid = netlist.graph.add_node(StdCellType::Symbol(symbol.into()));
            let iid = *symbol_to_nid
                .get(symbol)
                .ok_or(format!("output {} not found", symbol))?;
            netlist.graph.add_edge(oid, iid, ());
            netlist.roots.push(oid);
        }
    }
    Ok((netlist, parsed_module.name.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::liberty::{get_direction_of_pins, read_liberty};
    use petgraph::dot::{Config, Dot};
    use petgraph::graph::NodeIndex;
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

    #[test]
    fn test_read_verilog_with_lib_to_netlist_mul4_genus() {
        let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
        let lib = get_direction_of_pins(&liberty).unwrap();
        let (netlist, name) = read_verilog_with_lib_to_netlist("test/mul4_map_genus.v", lib).unwrap();
        assert_eq!(name, "Multiplier");
        fs::write(
            env::current_dir().unwrap().join("dot/test_mul4_map_genus_v.dot"),
            format!(
                "{:?}",
                Dot::with_config(&netlist.graph, &[Config::EdgeNoLabel])
            ),
        )
            .unwrap();
    }

    #[test]
    fn test_read_verilog_with_lib_to_netlist_add2_abc() {
        let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
        let lib = get_direction_of_pins(&liberty).unwrap();
        let (netlist, name) = read_verilog_with_lib_to_netlist("test/add2_map_abc.v", lib).unwrap();
        assert_eq!(name, "add2");
        fs::write(
            env::current_dir().unwrap().join("dot/test_add2_map_abc_v.dot"),
            format!(
                "{:?}",
                Dot::with_config(&netlist.graph, &[Config::EdgeNoLabel])
            ),
        )
            .unwrap();
        assert_eq!(netlist.leaves, vec![NodeIndex::new(0), NodeIndex::new(1), NodeIndex::new(2), NodeIndex::new(3)]);
        assert_eq!(netlist.roots, vec![NodeIndex::new(16), NodeIndex::new(17), NodeIndex::new(18)]);
    }
}
