use ezcp::alldifferent::AllDifferentConstraint;
use ezcp::linear::LinearInequalityConstraint;
use ezcp::objective_function::ObjectiveFunction;
use ezcp::solver::Solver;
use ezcp::value_selector::MinValueSelector;
use ezcp::variable::Variable;
use ezcp::variable_selector::FirstFailVariableSelector;
use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

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
    assert!(solver.solve());
    assert!(solver.get_objective() == 45);
}
