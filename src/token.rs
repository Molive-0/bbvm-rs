use lazy_static::lazy_static;
use regex::Regex;
use std::{rc::Rc, str::FromStr};

use crate::convert::Converter;

macro_rules! matches_token {
    ($str:literal, $i:ident) => {
        impl TokenImpl for $i {
            fn identify(ident: &str) -> bool {
                lazy_static! {
                    static ref RE: Regex = Regex::new($str).unwrap();
                }
                RE.is_match(ident)
            }
        }
    };
}

macro_rules! statement_token {
    ($ex:expr, $i:ident) => {
        impl TokenImpl for $i {
            fn identify(ident: &str) -> bool {
                const STATEMENTS: &[&str] = &$ex;
                STATEMENTS
                    .iter()
                    .find(|s| ***s == ident.to_lowercase())
                    .is_some()
            }
        }
    };
}

pub trait TokenImpl {
    fn identify(ident: &str) -> bool {
        false
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Token {
    Number(Number),
    Identifier(Identifier),
    While(While),
    TwoParam(TwoParam),
    OneParam(OneParam),
    Fluff,
    End,
    EOF,
}
pub trait StatementImpl {
    fn getVariables(&self) -> Vec<Rc<str>> {
        vec![]
    }
    fn compile(&self, cont: Rc<Converter>) -> () {}
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Statement {
    While(While),
    TwoParam(TwoParam),
    OneParam(OneParam),
    Fluff,
    End,
    EOF,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identifier {
    ident: Rc<str>,
}

matches_token!("[a-zA-Z]\\w*", Identifier);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Number {
    value: i128,
}

matches_token!("\\d+", Number);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct While {
    param: Identifier,
    num: Number,
}

impl StatementImpl for While {
    fn getVariables(&self) -> Vec<Rc<str>> {
        vec![self.param.ident.clone()]
    }
    fn compile(&self, cont: Rc<Converter>) -> () {
        cont.addWhile(self.param.ident.clone(), self.num.value.clone());
    }
}

impl TokenImpl for While {
    fn identify(ident: &str) -> bool {
        ident.to_lowercase().eq("while")
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TwoParamType {
    Copy,
}

impl FromStr for TwoParamType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "copy" => Ok(Self::Copy),
            _ => Err(()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TwoParam {
    one: Identifier,
    two: Identifier,
    ty: TwoParamType,
}

impl StatementImpl for TwoParam {
    fn getVariables(&self) -> Vec<Rc<str>> {
        vec![self.one.ident, self.two.ident]
    }
    fn compile(&self, cont: Rc<Converter>) -> () {
        match self.ty {
            TwoParamType::Copy => cont.addCopy(self.one.ident, self.two.ident),
        }
    }
}

statement_token!(["copy"], TwoParam);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OneParamType {
    Clear,
    Decr,
    Incr,
}

impl FromStr for OneParamType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "clear" => Ok(Self::Clear),
            "decr" => Ok(Self::Decr),
            "incr" => Ok(Self::Incr),
            _ => Err(()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OneParam {
    one: Identifier,
    ty: OneParamType,
}

impl StatementImpl for OneParam {
    fn getVariables(&self) -> Vec<Rc<str>> {
        vec![self.one.ident]
    }
    fn compile(&self, cont: Rc<Converter>) -> () {
        match self.ty {
            OneParamType::Clear => cont.addClear(self.one.ident),
            OneParamType::Decr => cont.addDecr(self.one.ident),
            OneParamType::Incr => cont.addIncr(self.one.ident),
        }
    }
}

statement_token!(["copy", "decr", "incr"], OneParam);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fluff {}

statement_token!(["do", "not", "to"], Fluff);

impl StatementImpl for Fluff {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct End {}

statement_token!(["end"], End);

impl StatementImpl for End {
    fn compile(&self, cont: Rc<Converter>) -> () {
        cont.addEnd()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EOF {}

impl TokenImpl for EOF {
    fn identify(ident: &str) -> bool {
        true
    }
}

impl StatementImpl for EOF {
    fn compile(&self, cont: Rc<Converter>) -> () {
        cont.addEOF()
    }
}
