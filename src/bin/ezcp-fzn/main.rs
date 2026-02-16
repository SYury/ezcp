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
    let get_var = |name: &str| {
        mz.solver
            .get_variable_by_name(name)
            .unwrap_or_else(|| panic!("Failed to find output variable {}", name))
            .borrow()
            .value()
    };
    for item in &mz.output {
        match item {
            Output::Var(name) => {
                println!("{} = {};", name, get_var(name));
            }
            Output::Array((name, a)) => {
                print!("{} = array1d(1..{},", name, a.len());
                for (i, var) in a.iter().enumerate() {
                    print!("{}{}", if i == 0 { '[' } else { ',' }, get_var(var));
                }
                println!("]);");
            }
        }
    }
    println!("----------");
    println!("==========");
}
