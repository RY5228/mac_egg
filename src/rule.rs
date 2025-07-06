use crate::language::*;
use egg::{Rewrite, rewrite};

pub fn make_aig_rules() -> Vec<Rewrite<AigLanguage, ()>> {
    let mut rules = vec![
        rewrite!("Negation_1"; "(not true)" => "false"),
        rewrite!("Negation_2"; "(not false)" => "true"),
        rewrite!("Simplify_1"; "(and true ?x)" => "?x"),
        rewrite!("Simplify_2"; "(and false ?x)" => "false"),
        rewrite!("Simplify_3"; "(and ?x true)" => "?x"),
        rewrite!("Simplify_4"; "(and ?x false)" => "false"),
        rewrite!("Simplify_5"; "(and ?x ?x)" => "?x"),
        rewrite!("Simplify_6"; "(and ?x (not ?x))" => "false"),
    ];
    rules.extend(
        vec![
            rewrite!("Commutative"; "(and ?x ?y)" <=> "(and ?y ?x)"),
            rewrite!("Associative"; "(and ?x (and ?y ?z))" <=> "(and (and ?x ?y) ?z)"),
            rewrite!("DoubleNegation"; "(not (not ?x))" <=> "?x"),
        ]
        .concat(),
    );
    rules
}
