use ezcp::alldifferent::AllDifferentConstraint;
use ezcp::cmp::NeqConstraint;
use ezcp::config::Config;
use ezcp::linear::LinearInequalityConstraint;
use ezcp::objective_function::ObjectiveFunction;
use ezcp::solver::Solver;
use ezcp::variable::Variable;
use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;

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
    let mut solver = Solver::new();
    let mut vars = Vec::with_capacity(10);
    for i in 0..10 {
        vars.push(solver.new_variable(0, 20, format!("var_{}", i)));
    }
    let ad = Box::new(AllDifferentConstraint::new(vars.clone()));
    solver.add_constraint(ad);
    for i in 0..9 {
        solver.add_constraint(Box::new(LinearInequalityConstraint::new(
            vec![vars[i].clone(), vars[i + 1].clone()],
            vec![1, -1],
            0,
        )));
    }
    let obj = Box::new(SumObjective { vars });
    solver.add_objective(obj);
    let mut search = solver.search(Config::default()).unwrap();
    if let Some(obj) = search.next() {
        assert!(obj == 45);
    } else {
        panic!("The solution is not optimal!");
    }
}

#[test]
fn test_time_limit_all_solutions() {
    let mut solver = Solver::new();
    let x = solver.new_variable(0, 1, format!("x"));
    let y = solver.new_variable(0, 1, format!("y"));
    solver.add_constraint(Box::new(NeqConstraint::new(x.clone(), y.clone())));
    let mut config = Config::default();
    config.all_solutions = true;
    config.time_limit = Some(200);
    let mut search = solver.search(config).unwrap();
    let fst = search.next();
    assert!(fst.is_some());
    sleep(Duration::from_millis(400));
    let snd = search.next();
    assert!(snd.is_some());
    let end = search.next();
    assert!(end.is_none());
    assert!(search.get_stats().borrow().whole_tree_explored);
}
