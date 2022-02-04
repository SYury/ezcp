use ezcp::binpacking::BinPackingConstraint;
use ezcp::solver::{binary_search_optimizer, Solver};
use ezcp::value_selector::MinValueSelector;
use ezcp::variable_selector::FirstFailVariableSelector;
use std::boxed::Box;
use std::fs::File;
use std::io::{BufRead, BufReader};

// read dataset in BPP format
fn read_dataset(filename: &str) -> (Vec<i64>, i64) {
    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);
    let mut lines = reader.lines().map(|l| l.unwrap());
    let n_items = lines.next().unwrap().parse::<usize>().unwrap();
    let capacity = lines.next().unwrap().parse::<i64>().unwrap();
    let mut items = Vec::with_capacity(n_items);
    for l in lines {
        items.push(l.parse::<i64>().unwrap());
    }
    (items, capacity)
}

// arguments: <file name in BPP format>
// use sample.txt as example dataset
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (items, capacity) = read_dataset(&args[1]);
    let create_solver = |bins: i64| {
        let mut solver = Solver::new(
            Box::new(FirstFailVariableSelector {}),
            Box::new(MinValueSelector {}),
        );
        let mut assignment = Vec::with_capacity(items.len());
        let mut load = Vec::with_capacity(bins as usize);
        for i in 0..items.len() {
            assignment.push(solver.new_variable(0, bins - 1, format!("assignment_{}", i)));
        }
        for i in 0..bins {
            load.push(solver.new_variable(0, capacity, format!("load_{}", i)));
        }
        let bp = Box::new(BinPackingConstraint::new(
            assignment.clone(),
            load.clone(),
            items.clone(),
        ));
        solver.add_constraint(bp);
        solver
    };
    let opt = binary_search_optimizer(create_solver, 0, items.len() as i64);
    println!("Optimal number of bins is {}", opt);
}
