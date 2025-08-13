use egg::{Id, Language, Symbol, define_language};
use std::fmt::Debug;
use std::hash::Hash;

pub trait LanguageType: Debug + Clone + PartialEq + Eq + Hash {
    type Lang: Language;

    fn from_lang(lang: Self::Lang) -> Self;

    fn from_op(op: &str) -> Self;
}

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
    Symbol(Symbol),
}

impl LanguageType for AigType {
    type Lang = AigLanguage;

    fn from_lang(lang: Self::Lang) -> Self {
        match lang {
            AigLanguage::Bool(b) => AigType::Bool(b),
            AigLanguage::And(_) => AigType::And,
            AigLanguage::Not(_) => AigType::Not,
            AigLanguage::Output(s, _) => AigType::Symbol(s),
            AigLanguage::Input(s) => AigType::Symbol(s),
            AigLanguage::Symbol(s) => AigType::Symbol(s),
        }
    }

    fn from_op(op: &str) -> Self {
        match op {
            "and" => AigType::And,
            "not" => AigType::Not,
            "true" => AigType::Bool(true),
            "false" => AigType::Bool(false),
            s => AigType::Symbol(s.to_owned().into()),
        }
    }
}

impl From<AigLanguage> for AigType {
    fn from(aig: AigLanguage) -> Self {
        AigType::from_lang(aig)
    }
}

define_language! {
    pub enum StdCellLanguage {
        Bool(bool),
        Gate(Symbol, Vec<Id>),
        Output(Symbol, Id),
        Input(Symbol),
        Symbol(Symbol),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StdCellType {
    Bool(bool),
    Symbol(Symbol),
}

impl LanguageType for StdCellType {
    type Lang = StdCellLanguage;
    
    fn from_lang(lang: StdCellLanguage) -> Self {
        match lang {
            StdCellLanguage::Bool(b) => { StdCellType::Bool(b) }
            StdCellLanguage::Gate(s, _) => { StdCellType::Symbol(s) }
            StdCellLanguage::Output(s, _) => { StdCellType::Symbol(s) }
            StdCellLanguage::Input(s) => { StdCellType::Symbol(s) }
            StdCellLanguage::Symbol(s) => { StdCellType::Symbol(s) }
        }
    }
    
    fn from_op(op: &str) -> Self {
        match op { 
            "true" => StdCellType::Bool(true),
            "false" => StdCellType::Bool(false),
            s => StdCellType::Symbol(s.to_owned().into()),
        }
    }
}