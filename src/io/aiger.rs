use crate::language::AigLanguage;
use crate::netlist::CombinitionalNetlist;
use aiger::{Aiger, AigerError, Literal, Reader, RecordsIter};
use egg::Id;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use std::fs::File;
use std::ops::Index;
use std::{env, io};

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

pub fn read_aag(path: &str) -> Result<CombinitionalNetlist<AigLanguage, ()>, ReadError> {
    fn handle_inverted(
        input: Literal,
        netlist: &mut CombinitionalNetlist<AigLanguage, ()>,
        variable_to_nid: &mut FxHashMap<usize, Id>,
    ) -> Id {
        if input.is_inverted() {
            netlist
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
    let mut netlist: CombinitionalNetlist<AigLanguage, ()> = Default::default();
    let mut variable_to_nid =
        FxHashMap::with_capacity_and_hasher(max_num_nodes, Default::default());

    let nid = netlist.egraph.add(AigLanguage::Bool(false));
    variable_to_nid.insert(0, nid);

    for input in aiger_data.inputs {
        if input.is_inverted() {
            return Err(ReadError::InvalidInverted);
        }
        let symbol = if let Some(symbol) = aiger_data.symbols.get(&input) {
            if symbol.type_spec != aiger::Symbol::Input {
                return Err(ReadError::InvalidInputSymbol);
            }
            AigLanguage::Symbol((&symbol.symbol).into())
        } else {
            AigLanguage::Symbol(format!("{:?}", input.variable()).into())
        };
        let nid = netlist.egraph.add(symbol);
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
        let input_1 = handle_inverted(gate.inputs[0], &mut netlist, &mut variable_to_nid);
        let input_2 = handle_inverted(gate.inputs[1], &mut netlist, &mut variable_to_nid);
        let nid = netlist.egraph.add(AigLanguage::And([input_1, input_2]));
        variable_to_nid.insert(gate.output.variable(), nid);
    }
    for output in aiger_data.outputs {
        if !variable_to_nid.contains_key(&output.variable()) {
            return Err(ReadError::InvalidLiteral);
        }
        let nid = handle_inverted(output, &mut netlist, &mut variable_to_nid);
        let symbol = if let Some(symbol) = aiger_data.symbols.get(&output) {
            if symbol.type_spec != aiger::Symbol::Output {
                return Err(ReadError::InvalidOutputSymbol);
            }
            AigLanguage::Output((&symbol.symbol).into(), nid)
        } else {
            AigLanguage::Output(format!("{:?}", output.variable()).into(), nid)
        };
        let nid = netlist.egraph.add(symbol);
        variable_to_nid.insert(output.variable(), nid);
        netlist.roots.push(nid);
    }
    netlist.egraph.rebuild();
    Ok(netlist)
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

#[test]
fn test_read_add2_aag() {
    let netlist = read_aag("test/add2.aag").unwrap();
    use crate::egg_to_serialized_egraph;
    let s = egg_to_serialized_egraph(&netlist.egraph, &netlist.roots);
    s.to_json_file(env::current_dir().unwrap().join("json/test_add2.json"))
        .unwrap();
    #[cfg(target_os = "linux")]
    s.to_svg_file(env::current_dir().unwrap().join("svg/test_add2.svg"))
        .unwrap();
}

#[test]
fn test_read_add1_aag() {
    let netlist = read_aag("test/add1.aag").unwrap();
    use crate::egg_to_serialized_egraph;
    let s = egg_to_serialized_egraph(&netlist.egraph, &netlist.roots);
    s.to_json_file(env::current_dir().unwrap().join("json/test_add1.json"))
        .unwrap();
    #[cfg(target_os = "linux")]
    s.to_svg_file(env::current_dir().unwrap().join("svg/test_add1.svg"))
        .unwrap();
}

#[test]
fn test_read_true_aag() {
    let netlist = read_aag("test/true.aag").unwrap();
    use crate::egg_to_serialized_egraph;
    let s = egg_to_serialized_egraph(&netlist.egraph, &netlist.roots);
    s.to_json_file(env::current_dir().unwrap().join("json/test_true.json")).unwrap();
    #[cfg(target_os = "linux")]
    s.to_svg_file(env::current_dir().unwrap().join("svg/test_true.svg")).unwrap();
}

#[test]
fn test_read_false_aag() {
    let netlist = read_aag("test/false.aag").unwrap();
    use crate::egg_to_serialized_egraph;
    let s = egg_to_serialized_egraph(&netlist.egraph, &netlist.roots);
    s.to_json_file(env::current_dir().unwrap().join("json/test_false.json"))
        .unwrap();
    #[cfg(target_os = "linux")]
    s.to_svg_file(env::current_dir().unwrap().join("svg/test_false.svg"))
}