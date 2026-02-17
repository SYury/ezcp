mod parser;

use crate::parser::{parse, Output};
use clap::Parser;
use ezcp::solver::SolutionStatus;
use std::fs::File;

#[derive(Parser, Debug)]
struct Args {
    model: String,
}

fn main() {
    let args = Args::parse();
    let file = File::open(args.model).expect("Failed reading flatzinc-json");
    let mut mz = parse(serde_json::from_reader(file).expect("Failed reading flatzinc-json"))
        .expect("Flatzinc-json parse error");
    let status = mz.solver.solve();
    if status == SolutionStatus::Infeasible {
        println!("=====UNSATISFIABLE=====");
        return;
    }
    for item in &mz.output {
        match item {
            Output::Var(var) => {
                let v = var.borrow();
                println!("{} = {};", &v.name, v.value());
            }
            Output::Array((name, a)) => {
                print!("{} = array1d(1..{},", name, a.len());
                for (i, var) in a.iter().enumerate() {
                    print!("{}{}", if i == 0 { '[' } else { ',' }, var.borrow().value());
                }
                println!("]);");
            }
        }
    }
    println!("----------");
    println!("==========");
}
