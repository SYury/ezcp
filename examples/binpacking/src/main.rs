use ezcp::binpacking::BinPackingConstraint;
use ezcp::solver::Solver;
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

// arguments: <file name in BPP format> <number of bins>
// use sample.txt as example dataset
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bins = args[2].parse::<usize>().unwrap();
    let (items, capacity) = read_dataset(&args[1]);
    let mut solver = Solver::new(
        Box::new(FirstFailVariableSelector {}),
        Box::new(MinValueSelector {}),
    );
    let mut assignment = Vec::with_capacity(items.len());
    let mut load = Vec::with_capacity(bins);
    for i in 0..items.len() {
        assignment.push(solver.new_variable(0, (bins as i64) - 1, format!("assignment_{}", i)));
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
    if !solver.solve() {
        println!("No solution!");
    } else {
        let mut real_load = vec![0; bins];
        for i in 0..assignment.len() {
            let var = assignment[i].borrow();
            assert!(var.is_assigned());
            let bin = var.value() as usize;
            real_load[bin] += items[i];
            println!("item {} goes to bin {}", i, bin);
        }
        for i in 0..bins {
            assert!(real_load[i] <= capacity);
        }
    }
}
