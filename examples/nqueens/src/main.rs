/* This program solves the N queens puzzle.
 *
 * Input format:
 * N (board side and number of queens)
 *
 * Output format:
 * N pairs (row, column)
 */
use ezcp::alldifferent::AllDifferentConstraint;
use ezcp::arithmetic::SimpleArithmeticConstraint;
use ezcp::solver::Solver;
use ezcp::value_selector::MinValueSelector;
use ezcp::variable_selector::FirstFailVariableSelector;
use std::boxed::Box;
use std::io;

fn read_int() -> usize {
    let mut input_line = String::new();
    io::stdin().read_line(&mut input_line).expect("No input!");
    input_line
        .trim()
        .parse()
        .expect("Input is not a valid unsigned integer!")
}

fn main() {
    let n = read_int();
    let mut solver = Solver::new(
        Box::new(FirstFailVariableSelector {}),
        Box::new(MinValueSelector {}),
    );
    let mut vars = Vec::with_capacity(n);
    let mut diag1 = Vec::with_capacity(n);
    let mut diag2 = Vec::with_capacity(n);
    for i in 0..n {
        vars.push(solver.new_variable(0, (n as i64) - 1, format!("pos_{}", i)));
        diag1.push(solver.new_variable(i as i64, (n + i - 1) as i64, format!("+diag_{}", i)));
        diag2.push(solver.new_variable(
            -(i as i64),
            (n as i64) - 1 - (i as i64),
            format!("-diag_{}", i),
        ));
        let d1 = Box::new(SimpleArithmeticConstraint::new(
            diag1[i].clone(),
            vars[i].clone(),
            i as i64,
            false,
        ));
        solver.add_constraint(d1);
        let d2 = Box::new(SimpleArithmeticConstraint::new(
            diag2[i].clone(),
            vars[i].clone(),
            -(i as i64),
            false,
        ));
        solver.add_constraint(d2);
    }
    let alldiff1 = Box::new(AllDifferentConstraint::new(vars.clone()));
    solver.add_constraint(alldiff1);
    let alldiff2 = Box::new(AllDifferentConstraint::new(diag1.clone()));
    solver.add_constraint(alldiff2);
    let alldiff3 = Box::new(AllDifferentConstraint::new(diag2.clone()));
    solver.add_constraint(alldiff3);
    assert!(solver.solve());
    let mut used = vec![false; n];
    let mut used_diag1 = vec![false; 2 * n];
    let mut used_diag2 = vec![false; 2 * n];
    for i in 0..n {
        assert!(vars[i].borrow().is_assigned());
        assert!(diag1[i].borrow().is_assigned());
        assert!(diag2[i].borrow().is_assigned());
        let val = vars[i].borrow().value() as usize;
        let val_d1 = diag1[i].borrow().value() as usize;
        let val_d2 = (diag2[i].borrow().value() + n as i64) as usize;
        println!("{} {}", i, val);
        assert!(val + i == val_d1);
        assert!(val + n == val_d2 + i);
        assert!(!used[val]);
        assert!(!used_diag1[val_d1]);
        assert!(!used_diag2[val_d2]);
        used[val] = true;
        used_diag1[val_d1] = true;
        used_diag2[val_d2] = true;
    }
}
