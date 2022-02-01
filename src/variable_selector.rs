use crate::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

pub trait VariableSelector {
    fn select(&self, vars: Vec<Rc<RefCell<Variable>>>) -> Rc<RefCell<Variable>>;
}

pub struct LexVariableSelector {}

impl VariableSelector for LexVariableSelector {
    fn select(&self, vars: Vec<Rc<RefCell<Variable>>>) -> Rc<RefCell<Variable>> {
        vars[0].clone()
    }
}

pub struct FirstFailVariableSelector {}

impl VariableSelector for FirstFailVariableSelector {
    fn select(&self, vars: Vec<Rc<RefCell<Variable>>>) -> Rc<RefCell<Variable>> {
        let mut pos = 0;
        let mut best_size = vars[0].borrow().size();
        for i in 1..vars.len() {
            let size = vars[i].borrow().size();
            if size < best_size {
                pos = i;
                best_size = size;
            }
        }
        vars[pos].clone()
    }
}
