mod parser;

use crate::parser::parse;
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
    for name in &mz.output {
        let val = mz
            .solver
            .get_variable_by_name(name)
            .unwrap_or_else(|| panic!("Failed to find output variable {}", name))
            .borrow()
            .value();
        println!("{} = {};", name, val);
    }
    println!("----------");
    println!("==========");
}
