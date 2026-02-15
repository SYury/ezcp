use crate::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

// function to minimize
pub trait ObjectiveFunction {
    // evaluates the objective assuming that all variables are set
    fn eval(&self) -> i64;
    // computes the best lower bound on the objective for the current variable domains
    fn bound(&self) -> i64;
}

pub struct SingleVariableObjective {
    pub var: Rc<RefCell<Variable>>,
    pub coeff: i64,
}

impl ObjectiveFunction for SingleVariableObjective {
    fn eval(&self) -> i64 {
        self.var.borrow().value() * self.coeff
    }

    fn bound(&self) -> i64 {
        if self.coeff > 0 {
            self.var.borrow().get_lb() * self.coeff
        } else {
            self.var.borrow().get_ub() * self.coeff
        }
    }
}
