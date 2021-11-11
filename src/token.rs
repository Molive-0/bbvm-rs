use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;

use crate::convert::Converter;

macro_rules! matches_token {
    ($str:literal, $i:ty) => {
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
    ($ex:expr, $i:ty) => {
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Token<'b> {
    Number(Number),
    Identifier(Identifier<'b>),
    While(While<'b>),
    TwoParam(TwoParam<'b>),
    OneParam(OneParam<'b>),
    Fluff,
    End,
    EOF,
}
pub trait StatementImpl<'a> {
    fn get_variables(&self) -> Vec<&'a str> {
        vec![]
    }
    fn compile(&self, _: &mut Converter<'a>) -> () {}
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Statement<'b> {
    While(While<'b>),
    TwoParam(TwoParam<'b>),
    OneParam(OneParam<'b>),
    Fluff,
    End,
    EOF,
}

impl<'a> TryFrom<Token<'a>> for Statement<'a> {
    fn try_from(t: Token<'a>) -> Result<Self, Self::Error> {
        use Token::*;
        match t {
            Number(v) => Err(format!("{:?} is not a statement!", v)),
            Identifier(v) => Err(format!("{:?} is not a statement!", v)),
            While(v) => Ok(Statement::While(v)),
            OneParam(v) => Ok(Statement::OneParam(v)),
            TwoParam(v) => Ok(Statement::TwoParam(v)),
            Fluff => Ok(Statement::Fluff),
            End => Ok(Statement::End),
            EOF => Ok(Statement::EOF),
        }
    }

    type Error = String;
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Identifier<'b> {
    pub ident: &'b str,
}

matches_token!("[a-zA-Z]\\w*", Identifier<'_>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Number {
    pub value: i128,
}

matches_token!("\\d+", Number);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct While<'b> {
    pub param: Identifier<'b>,
    pub num: Number,
}

impl<'a> StatementImpl<'a> for While<'a> {
    fn get_variables(&self) -> Vec<&'a str> {
        vec![self.param.ident]
    }
    fn compile(&self, cont: &mut Converter<'a>) -> () {
        cont.add_while(self.param.ident, self.num.value);
    }
}

impl<'b> TokenImpl for While<'b> {
    fn identify(ident: &str) -> bool {
        ident.to_lowercase().eq("while")
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct TwoParam<'b> {
    pub one: Identifier<'b>,
    pub two: Identifier<'b>,
    pub ty: TwoParamType,
}

impl<'a> StatementImpl<'a> for TwoParam<'a> {
    fn get_variables(&self) -> Vec<&'a str> {
        vec![self.one.ident, self.two.ident]
    }
    fn compile(&self, cont: &mut Converter<'a>) -> () {
        match self.ty {
            TwoParamType::Copy => cont.add_copy(self.one.ident, self.two.ident),
        }
    }
}

statement_token!(["copy"], TwoParam<'_>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct OneParam<'b> {
    pub one: Identifier<'b>,
    pub ty: OneParamType,
}

impl<'a> StatementImpl<'a> for OneParam<'a> {
    fn get_variables(&self) -> Vec<&'a str> {
        vec![self.one.ident]
    }
    fn compile(&self, cont: &mut Converter<'a>) -> () {
        match self.ty {
            OneParamType::Clear => cont.add_clear(self.one.ident),
            OneParamType::Decr => cont.add_decr(self.one.ident),
            OneParamType::Incr => cont.add_incr(self.one.ident),
        }
    }
}

statement_token!(["clear", "decr", "incr"], OneParam<'_>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Fluff {}

statement_token!(["do", "not", "to"], Fluff);

impl StatementImpl<'_> for Fluff {}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct End {}

statement_token!(["end"], End);

impl StatementImpl<'_> for End {
    fn compile(&self, cont: &mut Converter) -> () {
        cont.add_end()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct EOF {}

impl TokenImpl for EOF {
    fn identify(_: &str) -> bool {
        true
    }
}

impl StatementImpl<'_> for EOF {
    fn compile(&self, cont: &mut Converter) -> () {
        cont.add_eof()
    }
}
