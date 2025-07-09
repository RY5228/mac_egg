use egg::{Id, Symbol, define_language};

define_language! {
    pub enum AigLanguage {
        Bool(bool),
        "and" = And([Id; 2]),
        "not" = Not(Id),
        Output(Symbol, Id),
        Input(Symbol),
        Symbol(Symbol),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AigType {
    Bool(bool),
    And,
    Not,
    Output(Symbol),
    Input(Symbol),
    Symbol(Symbol),
}

impl From<AigLanguage> for AigType {
    fn from(aig: AigLanguage) -> Self {
        match aig {
            AigLanguage::Bool(b) => AigType::Bool(b),
            AigLanguage::And(_) => AigType::And,
            AigLanguage::Not(_) => AigType::Not,
            AigLanguage::Output(s, _) => AigType::Output(s),
            AigLanguage::Input(s) => AigType::Input(s),
            AigLanguage::Symbol(s) => AigType::Symbol(s),
        }
    }
}