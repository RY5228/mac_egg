use crate::egraph_roots::EGraphRoots;
use crate::language::{AigLanguage, AigType};
use crate::netlist::Netlist;
use aiger::{Aiger, AigerError, Literal, Reader, RecordsIter};
use egg::Id;
use indexmap::IndexMap;
use petgraph::Incoming;
use petgraph::graph::NodeIndex;
use rustc_hash::FxHashMap;
use std::fs::File;
use std::io;

#[derive(Debug)]
pub enum ReadError {
    NotImplementedForLatch,
    InvalidInverted,
    InvalidLiteral,
    InvalidInputSymbol,
    InvalidOutputSymbol,
    SymbolPositionOutOfBound,
    AigerError(AigerError),
    IoError(io::Error),
}

impl From<AigerError> for ReadError {
    fn from(_error: AigerError) -> Self {
        ReadError::AigerError(_error)
    }
}

impl From<io::Error> for ReadError {
    fn from(_error: io::Error) -> Self {
        ReadError::IoError(_error)
    }
}

pub fn read_aag_to_netlist(path: &str) -> Result<Netlist<AigType, ()>, ReadError> {
    fn handle_inverted(
        input: Literal,
        netlist: &mut Netlist<AigType, ()>,
        variable_to_nid: &mut FxHashMap<usize, NodeIndex>,
    ) -> NodeIndex {
        if input.is_inverted() {
            let input = variable_to_nid[&input.variable()];
            let output = netlist.graph.add_node(AigType::Not);
            netlist.graph.add_edge(output, input, ());
            output
        } else {
            variable_to_nid[&input.variable()]
        }
    }
    let file = File::open(path)?;
    let reader = Reader::from_reader(file)?;
    if reader.header().l > 0 {
        return Err(ReadError::NotImplementedForLatch);
    }
    let max_num_nodes = 2 * reader.header().m;
    let aiger_data = parse_aag(reader.records())?;
    let mut netlist: Netlist<AigType, ()> = Default::default();
    let mut variable_to_nid =
        FxHashMap::with_capacity_and_hasher(max_num_nodes, Default::default());

    let nid = netlist.graph.add_node(AigType::Bool(false));
    variable_to_nid.insert(0, nid);

    for input in aiger_data.inputs {
        if input.is_inverted() {
            return Err(ReadError::InvalidInverted);
        }
        let symbol = if let Some(symbol) = aiger_data.symbols.get(&input) {
            if symbol.type_spec != aiger::Symbol::Input {
                return Err(ReadError::InvalidInputSymbol);
            }
            AigType::Symbol((&symbol.symbol).into())
        } else {
            AigType::Symbol(format!("{:?}", input.variable()).into())
        };
        let nid = netlist.graph.add_node(symbol);
        variable_to_nid.insert(input.variable(), nid);
        netlist.leaves.push(nid);
    }
    for gate in aiger_data.gates {
        if !gate
            .inputs
            .iter()
            .all(|i| variable_to_nid.contains_key(&i.variable()))
        {
            return Err(ReadError::InvalidLiteral);
        }
        let iid_1 = handle_inverted(gate.inputs[0], &mut netlist, &mut variable_to_nid);
        let iid_2 = handle_inverted(gate.inputs[1], &mut netlist, &mut variable_to_nid);
        let oid = netlist.graph.add_node(AigType::And);
        netlist.graph.add_edge(oid, iid_1, ());
        netlist.graph.add_edge(oid, iid_2, ());
        variable_to_nid.insert(gate.output.variable(), oid);
    }
    for output in aiger_data.outputs {
        if !variable_to_nid.contains_key(&output.variable()) {
            return Err(ReadError::InvalidLiteral);
        }
        let iid = handle_inverted(output, &mut netlist, &mut variable_to_nid);
        let symbol = if let Some(symbol) = aiger_data.symbols.get(&output) {
            if symbol.type_spec != aiger::Symbol::Output {
                return Err(ReadError::InvalidOutputSymbol);
            }
            AigType::Symbol((&symbol.symbol).into())
        } else {
            AigType::Symbol(format!("{:?}", output.variable()).into())
        };
        let oid = netlist.graph.add_node(symbol);
        netlist.graph.add_edge(oid, iid, ());
        // println!("{:?} -> {:?}", oid, iid);
        // variable_to_nid.insert(output.variable(), oid);
        netlist.roots.push(oid);
    }
    let false_key = 0usize;
    let nid = variable_to_nid[&false_key];
    // println!("{:?}", variable_to_nid);
    // println!("{:?}", netlist.graph.edges_directed(nid, Incoming).count());
    // println!("{:?}", netlist.graph.edges_directed(nid, Outgoing).count());
    if netlist.graph.edges_directed(nid, Incoming).count() == 0 {
        netlist.graph.remove_node(nid); // None uses false node, remove from graph
    } else {
        netlist.leaves.push(nid); // Someone uses false node, add to leaves
    }

    Ok(netlist)
}

pub fn read_aag_to_egraph_roots(path: &str) -> Result<EGraphRoots<AigLanguage, ()>, ReadError> {
    fn handle_inverted(
        input: Literal,
        egraph_roots: &mut EGraphRoots<AigLanguage, ()>,
        variable_to_nid: &mut FxHashMap<usize, Id>,
    ) -> Id {
        if input.is_inverted() {
            egraph_roots
                .egraph
                .add(AigLanguage::Not(variable_to_nid[&input.variable()]))
        } else {
            variable_to_nid[&input.variable()]
        }
    }
    let file = File::open(path)?;
    let reader = Reader::from_reader(file)?;
    if reader.header().l > 0 {
        return Err(ReadError::NotImplementedForLatch);
    }
    let max_num_nodes = 2 * reader.header().m;
    let aiger_data = parse_aag(reader.records())?;
    let mut egraph_roots: EGraphRoots<AigLanguage, ()> = Default::default();
    let mut variable_to_nid =
        FxHashMap::with_capacity_and_hasher(max_num_nodes, Default::default());

    let nid = egraph_roots.egraph.add(AigLanguage::Bool(false));
    variable_to_nid.insert(0, nid);

    for input in aiger_data.inputs {
        if input.is_inverted() {
            return Err(ReadError::InvalidInverted);
        }
        let symbol = if let Some(symbol) = aiger_data.symbols.get(&input) {
            if symbol.type_spec != aiger::Symbol::Input {
                return Err(ReadError::InvalidInputSymbol);
            }
            AigLanguage::Input((&symbol.symbol).into())
        } else {
            AigLanguage::Input(format!("{:?}", input.variable()).into())
        };
        let nid = egraph_roots.egraph.add(symbol);
        variable_to_nid.insert(input.variable(), nid);
    }
    for gate in aiger_data.gates {
        if !gate
            .inputs
            .iter()
            .all(|i| variable_to_nid.contains_key(&i.variable()))
        {
            // let variables: Vec<usize> = gate.inputs.iter().map(|i| i.variable()).collect();
            // println!("{:?}", variables);
            // println!("{:?}", variable_to_nid);
            return Err(ReadError::InvalidLiteral);
        }
        let input_1 = handle_inverted(gate.inputs[0], &mut egraph_roots, &mut variable_to_nid);
        let input_2 = handle_inverted(gate.inputs[1], &mut egraph_roots, &mut variable_to_nid);
        let nid = egraph_roots
            .egraph
            .add(AigLanguage::And([input_1, input_2]));
        variable_to_nid.insert(gate.output.variable(), nid);
    }
    for output in aiger_data.outputs {
        if !variable_to_nid.contains_key(&output.variable()) {
            return Err(ReadError::InvalidLiteral);
        }
        let nid = handle_inverted(output, &mut egraph_roots, &mut variable_to_nid);
        let symbol = if let Some(symbol) = aiger_data.symbols.get(&output) {
            if symbol.type_spec != aiger::Symbol::Output {
                return Err(ReadError::InvalidOutputSymbol);
            }
            AigLanguage::Output((&symbol.symbol).into(), nid)
        } else {
            AigLanguage::Output(format!("{:?}", output.variable()).into(), nid)
        };
        let nid = egraph_roots.egraph.add(symbol);
        // variable_to_nid.insert(output.variable(), nid);
        egraph_roots.roots.push(nid);
    }
    egraph_roots.egraph.rebuild();
    Ok(egraph_roots)
}

#[derive(Debug)]
struct AndGate {
    output: Literal,
    inputs: [Literal; 2],
}

#[derive(Debug)]
struct Latch {
    output: Literal,
    input: Literal,
}

#[derive(Debug)]
struct Symbol {
    type_spec: aiger::Symbol,
    position: usize,
    symbol: String,
}

#[derive(Debug, Default)]
struct AigerData {
    inputs: Vec<Literal>,
    outputs: Vec<Literal>,
    gates: Vec<AndGate>,
    latches: Vec<Latch>,
    symbols: IndexMap<Literal, Symbol>,
}

fn parse_aag<T: io::Read>(records: RecordsIter<T>) -> Result<AigerData, ReadError> {
    let mut aiger_data = AigerData::default();
    for record in records {
        match record {
            Ok(Aiger::Input(input)) => {
                aiger_data.inputs.push(input);
            }
            Ok(Aiger::Output(output)) => {
                aiger_data.outputs.push(output);
            }
            Ok(Aiger::Symbol {
                type_spec,
                position,
                symbol,
            }) => {
                let l = match type_spec {
                    aiger::Symbol::Input => aiger_data.inputs.get(position),
                    aiger::Symbol::Output => aiger_data.outputs.get(position),
                    aiger::Symbol::Latch => return Err(ReadError::NotImplementedForLatch),
                }
                .ok_or(ReadError::SymbolPositionOutOfBound)?;
                aiger_data.symbols.insert(
                    *l,
                    Symbol {
                        type_spec,
                        position,
                        symbol,
                    },
                );
            }
            Ok(Aiger::AndGate { output, inputs }) => {
                aiger_data.gates.push(AndGate { output, inputs });
            }
            Ok(Aiger::Latch { output, input }) => {
                return Err(ReadError::NotImplementedForLatch);
                // aiger_data.latches.push(Latch { output, input });
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(aiger_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::dot::{Config, Dot};
    use std::env;
    #[test]
    fn test_read_add2_aag_egraph_roots() {
        let egraph_roots = read_aag_to_egraph_roots("test/add2.aag").unwrap();
        use crate::egg_to_serialized_egraph;
        let s = egg_to_serialized_egraph(&egraph_roots.egraph, &egraph_roots.roots);
        s.to_json_file(env::current_dir().unwrap().join("json/test_add2.json"))
            .unwrap();
        #[cfg(target_os = "linux")]
        s.to_svg_file(env::current_dir().unwrap().join("svg/test_add2.svg"))
            .unwrap();
    }

    #[test]
    fn test_read_add1_aag_egraph_roots() {
        let egraph_roots = read_aag_to_egraph_roots("test/add1.aag").unwrap();
        use crate::egg_to_serialized_egraph;
        let s = egg_to_serialized_egraph(&egraph_roots.egraph, &egraph_roots.roots);
        s.to_json_file(env::current_dir().unwrap().join("json/test_add1.json"))
            .unwrap();
        #[cfg(target_os = "linux")]
        s.to_svg_file(env::current_dir().unwrap().join("svg/test_add1.svg"))
            .unwrap();
    }

    #[test]
    fn test_read_true_aag_egraph_roots() {
        let egraph_roots = read_aag_to_egraph_roots("test/true.aag").unwrap();
        use crate::egg_to_serialized_egraph;
        let s = egg_to_serialized_egraph(&egraph_roots.egraph, &egraph_roots.roots);
        s.to_json_file(env::current_dir().unwrap().join("json/test_true.json"))
            .unwrap();
        #[cfg(target_os = "linux")]
        s.to_svg_file(env::current_dir().unwrap().join("svg/test_true.svg"))
            .unwrap();
    }

    #[test]
    fn test_read_false_aag_egraph_roots() {
        let egraph_roots = read_aag_to_egraph_roots("test/false.aag").unwrap();
        use crate::egg_to_serialized_egraph;
        let s = egg_to_serialized_egraph(&egraph_roots.egraph, &egraph_roots.roots);
        s.to_json_file(env::current_dir().unwrap().join("json/test_false.json"))
            .unwrap();
        // #[cfg(target_os = "linux")]
        s.to_svg_file(env::current_dir().unwrap().join("svg/test_false.svg"))
            .unwrap();
    }

    #[test]
    fn test_read_add2_aag_netlist() {
        let netlist = read_aag_to_netlist("test/add2.aag").unwrap();
        std::fs::write(
            env::current_dir().unwrap().join("dot/test_add2.dot"),
            format!(
                "{:?}",
                Dot::with_config(&netlist.graph, &[Config::EdgeNoLabel])
            ),
        )
        .unwrap();
    }

    #[test]
    fn test_read_add1_aag_netlist() {
        let netlist = read_aag_to_netlist("test/add1.aag").unwrap();
        std::fs::write(
            env::current_dir().unwrap().join("dot/test_add1.dot"),
            format!(
                "{:?}",
                Dot::with_config(&netlist.graph, &[Config::EdgeNoLabel])
            ),
        )
        .unwrap();
    }

    #[test]
    fn test_read_true_aag_netlist() {
        let netlist = read_aag_to_netlist("test/true.aag").unwrap();
        std::fs::write(
            env::current_dir().unwrap().join("dot/test_true.dot"),
            format!(
                "{:?}",
                Dot::with_config(&netlist.graph, &[Config::EdgeNoLabel])
            ),
        )
        .unwrap();
    }

    #[test]
    fn test_read_false_aag_netlist() {
        let netlist = read_aag_to_netlist("test/false.aag").unwrap();
        std::fs::write(
            env::current_dir().unwrap().join("dot/test_false.dot"),
            format!(
                "{:?}",
                Dot::with_config(&netlist.graph, &[Config::EdgeNoLabel])
            ),
        )
        .unwrap();
    }
}
