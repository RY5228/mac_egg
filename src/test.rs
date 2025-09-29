use std::env;
use anyhow::Context;
use egg::*;
use mac_egg::*;

#[test]
fn test_simple() {
    use egg_to_serialized_egraph;
    use rule::make_aig_rules;
    let runner = Runner::default()
        // .with_iter_limit(2)
        .with_expr(&"(and (and x y) (and x z))".parse().unwrap())
        // .with_expr(&"(xor3 x y z)".parse().unwrap())
        .run(&make_aig_rules());
    let s: SerializedEGraph = egg_to_serialized_egraph(&runner.egraph, &runner.roots);
    let (egraph, root) = (runner.egraph, runner.roots[0]);
    let mut extractors = extractor::extractors();
    extractors.retain(|_, ed| ed.get_use_for_bench());
    let extractor_name: String = "faster-greedy-dag".into();
    let ed = extractors
        .get(extractor_name.as_str())
        .with_context(|| format!("Unknown extractor: {extractor_name}"))
        .unwrap();
    let result = ed.get_extractor().extract(&s, &s.root_eclasses);
    result.check(&s);
    println!("{:?}", result.choices);

    let extractor = Extractor::new(&egraph, AstSize);
    let (best_cost, best) = extractor.find_best(root);
    println!("egg result:");
    println!("{:?}", best_cost);
    println!("{:?}", best);
    use std::env;
    s.to_json_file(
        env::current_dir()
            .unwrap()
            .join("json/serialized_egraph_test.json"),
    )
    .unwrap();
    #[cfg(target_os = "linux")]
    s.to_svg_file(
        env::current_dir()
            .unwrap()
            .join("svg/serialized_egraph_test.svg"),
    )
    .unwrap();
}

#[test]
fn test_mul32() {
    use egraph_roots::EGraphRoots;
    use io::liberty::{get_direction_of_pins, read_liberty};
    use io::stdcell::{read_verilog_with_lib_to_netlist, write_verilog_from_netlist_with_lib};
    use language::StdCellLanguage;
    use language::StdCellType;
    use rule::JsonRules;
    use choose_result_in_serialized_egraph_into_netlist;
    
    let liberty = read_liberty("test/asap7sc6t_SELECT_LVT_TT_nldm.lib").unwrap();
    let lib = get_direction_of_pins(&liberty).unwrap();
    let (netlist, name) = read_verilog_with_lib_to_netlist("test/mul32_map_genus.v", lib.clone()).unwrap();
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
    let runner = Runner::default()
        .with_egraph(egraph_roots.egraph)
        .run(&rules);
    let s = egg_to_serialized_egraph(&runner.egraph, &egraph_roots.roots);
    let mut extractors = extractor::extractors();
    extractors.retain(|_, ed| ed.get_use_for_bench());
    let extractor_name: String = "faster-greedy-dag".into();
    let ed = extractors
        .get(extractor_name.as_str())
        .with_context(|| format!("Unknown extractor: {extractor_name}"))
        .unwrap();
    let result = ed.get_extractor().extract(&s, &s.root_eclasses);
    result.check(&s);
    let new_netlist = choose_result_in_serialized_egraph_into_netlist::<StdCellType>(&s, &result).unwrap();
    // std::fs::write("verilog/test_mul32_map_genus_inv_dmg_rules_extract.v.netlist", format!("{:#?}", new_netlist)).unwrap();
    write_verilog_from_netlist_with_lib("verilog/test_mul32_map_genus_inv_dmg_rules_extract.v", new_netlist, &name, lib).unwrap();
}