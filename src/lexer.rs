use std::{rc::Rc, str::FromStr};

use crate::token::*;

macro_rules! incorrect {
    ($t:ident,$l:literal,$c:ident) => {
        panic!("Token {} should be followed by {}, not {:?}", $t, $l, $c)
    };
}

pub struct Lexer<'a> {
    input: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(input: &str) -> Lexer {
        Lexer { input }
    }

    fn get_not_fluff(&mut self) -> Token {
        let mut t = Token::Fluff;
        while Token::Fluff == t {
            t = self.get_token();
        }
        t
    }

    fn get_token(&mut self) -> Token {
        let token;
        loop {
            if self.input.is_empty() {
                return Token::EOF;
            }
            let split = self.input.trim_start().split_once(char::is_whitespace);
            if split.is_none() {
                return Token::EOF;
            }
            let (t, remaining) = split.unwrap();

            self.input = remaining;

            if t.starts_with("#") {
                if self.input.is_empty() {
                    return Token::EOF;
                }
                let split = self.input.split_once("\n");
                if split.is_none() {
                    return Token::EOF;
                }
                self.input = split.unwrap().1;
            } else if !t.is_empty() {
                token = t;
                break;
            }
        }

        if TwoParam::identify(token) {
            let get = self.get_not_fluff();
            if let Token::Identifier(one) = get {
                let get = self.get_not_fluff();
                if let Token::Identifier(two) = get {
                    Token::TwoParam(TwoParam {
                        one,
                        two,
                        ty: TwoParamType::from_str(token).unwrap(),
                    })
                } else {
                    incorrect!(token, "identifier", get);
                }
            } else {
                incorrect!(token, "identifier", get);
            }
        } else if OneParam::identify(token) {
            let get = self.get_not_fluff();
            if let Token::Identifier(one) = get {
                Token::OneParam(OneParam {
                    one,
                    ty: OneParamType::from_str(token).unwrap(),
                })
            } else {
                incorrect!(token, "identifier", get);
            }
        } else if While::identify(token) {
            let get = self.get_not_fluff();
            if let Token::Identifier(param) = get {
                let get = self.get_not_fluff();
                if let Token::Number(num) = get {
                    Token::While(While { param, num })
                } else {
                    incorrect!(token, "number", get);
                }
            } else {
                incorrect!(token, "identifier", get);
            }
        } else if Fluff::identify(token) {
            Token::Fluff
        } else if End::identify(token) {
            Token::End
        } else if Identifier::identify(token) {
            Token::Identifier(Identifier {
                ident: Rc::from(token),
            })
        } else if Number::identify(token) {
            Token::Number(Number {
                value: i128::from_str(token).unwrap(),
            })
        } else {
            Token::EOF
        }
    }
}
