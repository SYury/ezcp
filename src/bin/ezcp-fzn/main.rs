mod parser;

use crate::parser::{parse, Output};
use clap::Parser;
use std::fs::File;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short)]
    a: bool,
    #[arg(short)]
    n: Option<usize>,
    model: String,
}

fn main() {
    let args = Args::parse();
    let file = File::open(args.model).expect("Failed reading flatzinc-json");
    let mut mz = parse(serde_json::from_reader(file).expect("Failed reading flatzinc-json"))
        .expect("Flatzinc-json parse error");
    if args.a || args.n.is_some() {
        mz.config.all_solutions = true;
    }
    let search = mz.solver.search(mz.config).unwrap();
    let mut found = false;
    for (sid, _) in search.enumerate() {
        found = true;
        for item in &mz.output {
            match item {
                Output::Var(var) => {
                    let v = var.borrow();
                    if mz.bools.contains(&v.name) {
                        println!(
                            "{} = {};",
                            &v.name,
                            if v.value() == 0 { "false" } else { "true" }
                        );
                    } else {
                        println!("{} = {};", &v.name, v.value());
                    }
                }
                Output::Array((name, a)) => {
                    print!("{} = array1d(1..{},", name, a.len());
                    for (i, var) in a.iter().enumerate() {
                        let v = var.borrow();
                        if mz.bools.contains(&v.name) {
                            print!(
                                "{}{}",
                                if i == 0 { '[' } else { ',' },
                                if v.value() == 0 { "false" } else { "true" }
                            );
                        } else {
                            print!("{}{}", if i == 0 { '[' } else { ',' }, v.value());
                        }
                    }
                    println!("]);");
                }
            }
        }
        println!("----------");
        if Some(sid + 1) == args.n {
            break;
        }
    }
    if found {
        println!("==========");
    } else {
        println!("=====UNSATISFIABLE=====");
    }
}
