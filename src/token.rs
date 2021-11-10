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
    fn identify(_: &str) -> bool {
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
    fn get_variables(&self) -> Vec<Rc<str>> {
        vec![]
    }
    fn compile(&self, _: &mut Converter) -> () {}
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

impl TryFrom<Token> for Statement {
    fn try_from(t: Token) -> Result<Self, Self::Error> {
        use Token::*;
        match t {
            Number(_) => Err("Number is not a statement!"),
            Identifier(_) => Err("Identifier is not a statement!"),
            While(v) => Ok(Statement::While(v)),
            OneParam(v) => Ok(Statement::OneParam(v)),
            TwoParam(v) => Ok(Statement::TwoParam(v)),
            Fluff => Ok(Statement::Fluff),
            End => Ok(Statement::End),
            EOF => Ok(Statement::EOF),
        }
    }

    type Error = &'static str;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identifier {
    pub ident: Rc<str>,
}

matches_token!("[a-zA-Z]\\w*", Identifier);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Number {
    pub value: i128,
}

matches_token!("\\d+", Number);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct While {
    pub param: Identifier,
    pub num: Number,
}

impl StatementImpl for While {
    fn get_variables(&self) -> Vec<Rc<str>> {
        vec![self.param.ident.clone()]
    }
    fn compile(&self, cont: &mut Converter) -> () {
        cont.add_while(self.param.ident.clone(), self.num.value.clone());
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
    pub one: Identifier,
    pub two: Identifier,
    pub ty: TwoParamType,
}

impl StatementImpl for TwoParam {
    fn get_variables(&self) -> Vec<Rc<str>> {
        vec![self.one.ident.clone(), self.two.ident.clone()]
    }
    fn compile(&self, cont: &mut Converter) -> () {
        match self.ty {
            TwoParamType::Copy => cont.add_copy(self.one.ident.clone(), self.two.ident.clone()),
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
    pub one: Identifier,
    pub ty: OneParamType,
}

impl StatementImpl for OneParam {
    fn get_variables(&self) -> Vec<Rc<str>> {
        vec![self.one.ident.clone()]
    }
    fn compile(&self, cont: &mut Converter) -> () {
        match self.ty {
            OneParamType::Clear => cont.add_clear(self.one.ident.clone()),
            OneParamType::Decr => cont.add_decr(self.one.ident.clone()),
            OneParamType::Incr => cont.add_incr(self.one.ident.clone()),
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
    fn compile(&self, cont: &mut Converter) -> () {
        cont.add_end()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EOF {}

impl TokenImpl for EOF {
    fn identify(_: &str) -> bool {
        true
    }
}

impl StatementImpl for EOF {
    fn compile(&self, cont: &mut Converter) -> () {
        cont.add_eof()
    }
}
