/* This program solves SAT problem.
 *
 * Single command line argument:
 * path to file in DIMACS CNF format (https://www.cs.ubc.ca/~hoos/SATLIB/Benchmarks/SAT/satformat.ps)
 *
 * Use sample_satisfiable.cnf and sample_unsatisfiable.cnf for example (files taken from SATLIB: https://www.cs.ubc.ca/~hoos/SATLIB/benchm.html)
 */
use ezcp::brancher::MinValueBrancher;
use ezcp::config::Config;
use ezcp::logic::{AndConstraint, NegateConstraint, OrConstraint};
use ezcp::solver::{SolutionStatus, Solver};
use ezcp::variable_selector::FirstFailVariableSelector;
use std::boxed::Box;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn read_cnf_file(filename: &str) -> (usize, Vec<Vec<(usize, bool)>>) {
    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);
    let mut n_vars = 0;
    let mut n_clauses = 0;
    let mut clauses = vec![Vec::new(); 1];
    for line in reader.lines().map(|l| l.unwrap()) {
        if line.is_empty() {
            continue;
        }
        let begin = line.as_bytes()[0] as char;
        if begin == 'c' {
            continue;
        }
        if begin == 'p' {
            let tokens: Vec<_> = line.split(' ').filter(|x| !x.is_empty()).collect();
            assert!(tokens.len() == 4);
            if tokens[1] != "cnf" {
                panic!("Unsupported format: {}", tokens[1]);
            }
            n_vars = tokens[2].parse::<usize>().unwrap();
            n_clauses = tokens[3].parse::<usize>().unwrap();
            continue;
        }
        if !begin.is_ascii_digit() && begin != ' ' && begin != '-' {
            continue;
        }
        for literal in line.split(' ').filter(|x| !x.is_empty()).map(|x| x.parse::<i32>().unwrap()) {
            if literal == 0 {
                clauses.push(Vec::new());
            } else {
                if literal < 0 {
                    clauses.last_mut().unwrap().push(((-literal - 1) as usize, false));
                } else {
                    clauses.last_mut().unwrap().push(((literal - 1) as usize, true));
                }
            }
        }
    }
    clauses = clauses.iter().filter(|x| !x.is_empty()).cloned().collect();
    assert!(clauses.len() == n_clauses);
    (n_vars, clauses)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        panic!("You must provide a single argument: path to CNF file.");
    }
    let (n_vars, clauses) = read_cnf_file(&args[1]);
    let mut solver = Solver::new(
        Config::new(
            Box::new(MinValueBrancher {}),
            Box::new(FirstFailVariableSelector {}),
        )
    );
    let mut vars = Vec::with_capacity(n_vars);
    let mut negations = Vec::with_capacity(n_vars);
    let mut clause_vars = Vec::with_capacity(clauses.len());
    for i in 0..n_vars {
        let v = solver.new_variable(0, 1, format!("v_{}", i));
        let nv = solver.new_variable(0, 1, format!("not v_{}", i));
        vars.push(v.clone());
        negations.push(nv.clone());
        solver.add_constraint(Box::new(NegateConstraint::new(v, nv)));
    }
    for i in 0..clauses.len() {
        let cv = solver.new_variable(0, 1, format!("clause_{}", i));
        clause_vars.push(cv.clone());
        let mut v = Vec::new();
        for (id, neg) in clauses[i].iter().cloned() {
            if neg {
                v.push(negations[id].clone());
            } else {
                v.push(vars[id].clone());
            }
        }
        solver.add_constraint(Box::new(OrConstraint::new(cv, v)));
    }
    let sat_var = solver.new_variable(1, 1, format!("sat"));
    solver.add_constraint(Box::new(AndConstraint::new(sat_var.clone(), clause_vars.clone())));
    if solver.solve() == SolutionStatus::Infeasible {
        println!("Unsatisfiable.");
    } else {
        println!("Satisfiable.");
        for v in &vars {
            print!("{} ", v.borrow().value());
        }
        println!("");
    }
}
