use anyhow::Context;
use egg::*;
use mac_egg::*;

#[test]
fn test() {
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
