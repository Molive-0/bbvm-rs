#![feature(iter_zip)]

use crate::convert::Converter;
use crate::lexer::Lexer;
use crate::token::{Statement, StatementImpl};
use clap::{crate_authors, crate_description, crate_name, crate_version, App};
use inkwell::context::Context;
use std::fs;

mod convert;
mod lexer;
mod token;

fn main() -> () {
    let starttime = chrono::Utc::now();
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg("-c     'Tries to compile the code to native'")
        .arg("<INPUT>'Sets the input file to use'")
        .get_matches();

    let compile = matches.is_present("c");
    let filename = matches.value_of("INPUT").unwrap();

    let file = fs::read_to_string(filename).expect("Failed to read the file");
    let l = Lexer::new(&file);

    println!("Interpreting file...");
    let mut tokens: Vec<Statement> = vec![];
    loop {
        let statement = l.get_token().try_into().unwrap();
        if statement == Statement::EOF {
            tokens.push(statement);
            break;
        }
        tokens.push(statement);
    }

    let variables: Vec<&str> = tokens
        .iter()
        .flat_map(|t| {
            use Statement::*;
            match t {
                EOF | Fluff | End => {
                    vec![]
                }
                While(v) => v.get_variables(),
                OneParam(v) => v.get_variables(),
                TwoParam(v) => v.get_variables(),
            }
        })
        .collect();
    let context = Context::create();
    let mut converter = Converter::new(variables, &context);

    println!("Generating LLVM IR...");
    for statement in tokens {
        use Statement::*;
        match statement {
            Fluff => {}
            End => converter.add_end(),
            EOF => converter.add_eof(),
            While(v) => v.compile(&mut converter),
            OneParam(v) => v.compile(&mut converter),
            TwoParam(v) => v.compile(&mut converter),
        }
    }

    let endtime1 = chrono::Utc::now();
    let duration = endtime1 - starttime;

    println!(
        "LLVM IR compile took {} nanoseconds ({} milliseconds).",
        duration.num_nanoseconds().unwrap_or_default(),
        duration.num_milliseconds()
    );

    if compile {
        println!("Running normal compiler...");

        converter.dump_code();
    } else {
        println!("Running JIT compiler...");

        converter.run();
    }

    let endtime2 = chrono::Utc::now();
    let duration = endtime2 - endtime1;

    println!(
        "LLVM IR execution took {} nanoseconds ({} milliseconds).",
        duration.num_nanoseconds().unwrap_or_default(),
        duration.num_milliseconds()
    );

    if compile {
        println!("A compiled executable is available at ./bbvm.out");
    }
}
