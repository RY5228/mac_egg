use egg::{Id, Symbol, define_language};

define_language! {
    pub enum AigLanguage {
        Bool(bool),
        "and" = And([Id; 2]),
        "not" = Not(Id),
        Output(Symbol, Id),
        Symbol(Symbol),
    }
}
