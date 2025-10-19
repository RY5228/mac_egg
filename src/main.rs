use clap::{arg, Parser};
use clap_verbosity_flag::Verbosity;
use egg::Runner;
use egg::StopReason::Saturated;
use log::{info, warn};
use mac_egg::egraph_roots::EGraphRoots;
use mac_egg::io::liberty::{get_direction_of_pins, read_liberty};
use mac_egg::io::stdcell::read_verilog_with_lib_to_netlist;
use mac_egg::language::StdCellLanguage;
use mac_egg::rule::JsonRules;
use mac_egg::{egg_to_serialized_egraph, netlist_to_egg_roots};
use std::path::PathBuf;
use std::time::Duration;
use std::{env, fs};
use mac_egg::mining::GSpan;

/// Standard cell fusion by mining frequent subcircuits with egraph from a standard cell netlist.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path of the input netlist (.v)
    #[arg(short, long, value_name = "FILE")]
    input: PathBuf,
    /// Path of the library (.lib)
    #[arg(short, long, value_name = "FILE")]
    library: PathBuf,
    /// Directory of the outputs
    #[arg(short, long, value_name = "DIR")]
    output: PathBuf,
    /// Min support
    #[arg(long, default_value_t = 10)]
    min_support: usize,
    /// Max subcircuit size
    #[arg(long, default_value_t = 5)]
    max_size: usize,
    /// Max number of subcircuit inputs
    #[arg(long, default_value_t = 3)]
    max_num_inputs: usize,
    /// TopK frequent subcircuits
    #[arg(long, default_value_t = 10)]
    top_k: usize,
    /// Verbose log
    #[command(flatten)]
    verbose: Verbosity,
}

fn main() {
    let args = Args::parse();
    let manifest_dir: PathBuf = env::var("CARGO_MANIFEST_DIR").unwrap().into();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();

    let liberty = read_liberty(args.library).unwrap();
    let lib = get_direction_of_pins(&liberty).unwrap();
    let (netlist, name) = read_verilog_with_lib_to_netlist(args.input, lib.clone()).unwrap();
    let egraph_roots: EGraphRoots<_, ()> = netlist_to_egg_roots(&netlist).unwrap();
    let mut rules = JsonRules::from_path(manifest_dir.join("test/6t_inv_rules.json"))
        .unwrap()
        .into_egg_rules::<StdCellLanguage>()
        .unwrap();
    rules.extend(
        JsonRules::from_path(manifest_dir.join("test/6t_dmg_rules.json"))
            .unwrap()
            .into_egg_rules::<StdCellLanguage>()
            .unwrap(),
    );
    rules.extend(
        JsonRules::from_path(manifest_dir.join("test/6t_comm_rules.json"))
            .unwrap()
            .into_egg_rules::<StdCellLanguage>()
            .unwrap(),
    );
    info!(
        "Initial egraph has {} nodes.",
        egraph_roots.egraph.total_number_of_nodes()
    );
    let runner = Runner::default()
        .with_egraph(egraph_roots.egraph)
        .with_node_limit(1_000_000)
        .with_iter_limit(100)
        .with_time_limit(Duration::from_secs(30))
        .run(&rules);
    if let Some(Saturated) = runner.stop_reason {
        info!("Stop reason is {:?}.", runner.stop_reason);
    } else {
        warn!("Stop reason is {:?}.", runner.stop_reason);
    }
    info!(
        "Rewritten egraph has {} nodes.",
        runner.egraph.total_number_of_nodes()
    );

    let s = egg_to_serialized_egraph(&runner.egraph, &egraph_roots.roots);
    if !args.output.exists() {
        fs::create_dir_all(&args.output).unwrap();
        info!("Created output directory {}", args.output.display());
    }
    let rewritten_egraph_path = args.output.join("rewritten_egraph.json");
    s.to_json_file(&rewritten_egraph_path).unwrap();
    info!("Wrote rewritten egraph to {}", rewritten_egraph_path.display());

    let mut gspan = GSpan::new(s, lib, args.min_support, args.max_size, args.max_num_inputs).unwrap();
    gspan.mine();
    info!("Mined egraph.");
    info!("Got {} frequent patterns.", gspan.frequent_patterns().len());
    for (i, (code, support)) in gspan.top_frequent_patterns(args.top_k).iter().enumerate() {
        let mut blif = String::new();
        blif += format!("# support = {}\n", support).as_str();
        blif += code.to_blif(format!("fused_cell_{i}").as_str()).as_str();
        blif += "\n";
        let blif_path = args.output.join(format!("{i}.blif"));
        fs::write(&blif_path, blif).unwrap();
        info!("Wrote subcircuit with support {} to {}, ", support, blif_path.display());
    }
}

#[cfg(test)]
pub mod test;
