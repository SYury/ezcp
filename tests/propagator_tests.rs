use ezcp::alldifferent::{AllDifferentACPropagator, AllDifferentConstraint};
use ezcp::objective_function::ObjectiveFunction;
use ezcp::propagator::Propagator;
use ezcp::solver::{Solver, SolverState};
use ezcp::value_selector::MinValueSelector;
use ezcp::variable::Variable;
use ezcp::variable_selector::FirstFailVariableSelector;
use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

fn assert_domain(mut it: impl Iterator<Item = i64>, expected: Vec<i64>) {
    let mut it1 = expected.iter().cloned();
    loop {
        let x = it.next();
        let y = it1.next();
        if x.is_none() {
            if y.is_none() {
                break;
            } else {
                assert!(
                    false,
                    "Domain iterator ended, but expected value {}",
                    y.unwrap()
                );
            }
        }
        if y.is_none() {
            assert!(
                false,
                "Expected domain iterator to end, but got value {}",
                x.unwrap()
            );
        }
        let xval = x.unwrap();
        let yval = y.unwrap();
        assert_eq!(
            xval, yval,
            "Expected value {} in domain, but got {}",
            yval, xval
        );
    }
}

#[test]
fn test_alldifferent() {
    let fake_solver_state = Rc::new(RefCell::new(SolverState::new()));
    let x = Rc::new(RefCell::new(Variable::new(
        fake_solver_state.clone(),
        0,
        2,
        "x".to_string(),
    )));
    let y = Rc::new(RefCell::new(Variable::new(
        fake_solver_state.clone(),
        0,
        0,
        "y".to_string(),
    )));
    let z = Rc::new(RefCell::new(Variable::new(
        fake_solver_state,
        2,
        2,
        "z".to_string(),
    )));
    let mut p = AllDifferentACPropagator::new(vec![x.clone(), y.clone(), z.clone()], 0);
    p.propagate();
    assert_domain(x.borrow().iter(), vec![1]);
    assert_domain(y.borrow().iter(), vec![0]);
    assert_domain(z.borrow().iter(), vec![2]);
}

struct SumObjective {
    vars: Vec<Rc<RefCell<Variable>>>,
}

impl ObjectiveFunction for SumObjective {
    fn eval(&self) -> i64 {
        let mut sum = 0;
        for var in &self.vars {
            sum += var.borrow().value();
        }
        sum
    }

    fn bound(&self) -> i64 {
        let mut sum = 0;
        for var in &self.vars {
            sum += var.borrow().get_lb();
        }
        sum
    }
}

#[test]
fn test_optimization() {
    let mut solver = Solver::new(
        Box::new(FirstFailVariableSelector {}),
        Box::new(MinValueSelector {}),
    );
    let mut vars = Vec::with_capacity(10);
    for i in 0..10 {
        vars.push(solver.new_variable(i as i64, 20, format!("var_{}", i)));
    }
    let ad = Box::new(AllDifferentConstraint::new(vars.clone()));
    solver.add_constraint(ad);
    let obj = Box::new(SumObjective { vars });
    solver.add_objective(obj);
    assert!(solver.solve());
    assert!(solver.get_objective() == 45);
}
