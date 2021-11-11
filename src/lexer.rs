use std::{str::FromStr, sync::Mutex};

use crate::token::*;

macro_rules! incorrect {
    ($t:ident,$l:literal,$c:ident) => {
        panic!("Token {} should be followed by {}, not {:?}", $t, $l, $c)
    };
}

pub struct Lexer<'a> {
    input: Mutex<&'a str>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &str) -> Lexer {
        Lexer {
            input: Mutex::new(input),
        }
    }

    fn get_not_fluff(&self) -> Token {
        loop {
            let t = self.get_token();
            if t != Token::Fluff {
                return t;
            }
        }
    }

    pub fn get_token(&self) -> Token {
        let token;
        let mut input = self.input.lock().unwrap();
        loop {
            if input.is_empty() {
                return Token::EOF;
            }
            let split = input
                .trim_start()
                .split_once(|c: char| c.is_whitespace() || c == ';');
            if split.is_none() {
                return Token::EOF;
            }
            let (t, remaining) = split.unwrap();

            *input = remaining;

            if t.starts_with("#") {
                if input.is_empty() {
                    return Token::EOF;
                }
                let split = input.split_once("\n");
                if split.is_none() {
                    return Token::EOF;
                }
                *input = split.unwrap().1;
            } else if !t.is_empty() {
                token = t;
                break;
            }
        }
        drop(input);

        if TwoParam::identify(token) {
            let get = self.get_not_fluff().clone();
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
            Token::Identifier(Identifier { ident: token })
        } else if Number::identify(token) {
            Token::Number(Number {
                value: i128::from_str(token).unwrap(),
            })
        } else {
            Token::EOF
        }
    }
}
