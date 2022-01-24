use std::cell::RefCell;
use std::rc::Rc;
use crate::variable::Variable;

pub trait VariableSelector {
    fn select(&self, vars: Vec<Rc<RefCell<Variable>>>) -> Rc<RefCell<Variable>>;
}

pub struct LexVariableSelector {}

impl VariableSelector for LexVariableSelector {
    fn select(&self, vars: Vec<Rc<RefCell<Variable>>>) -> Rc<RefCell<Variable>> {
        vars[0].clone()
    }
}
