use crate::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

/// Branches on the given variable.
pub trait Brancher {
    fn n_branches(&self, v: Rc<RefCell<Variable>>) -> usize;
    fn branch(&self, v: Rc<RefCell<Variable>>, branch: usize);
}

pub struct MinValueBrancher {}

impl Brancher for MinValueBrancher {
    fn n_branches(&self, _: Rc<RefCell<Variable>>) -> usize {
        2
    }
    fn branch(&self, v: Rc<RefCell<Variable>>, branch: usize) {
        let mut vv = v.borrow_mut();
        let x = vv.get_lb();
        if branch == 0 {
            vv.assign(x);
        } else {
            vv.remove(x);
        }
    }
}

pub struct MaxValueBrancher {}

impl Brancher for MaxValueBrancher {
    fn n_branches(&self, _: Rc<RefCell<Variable>>) -> usize {
        2
    }
    fn branch(&self, v: Rc<RefCell<Variable>>, branch: usize) {
        let mut vv = v.borrow_mut();
        let x = vv.get_ub();
        if branch == 0 {
            vv.assign(x);
        } else {
            vv.remove(x);
        }
    }
}

pub struct SplitBrancher {
    reverse: bool,
}

impl Brancher for SplitBrancher {
    fn n_branches(&self, _: Rc<RefCell<Variable>>) -> usize {
        2
    }
    fn branch(&self, v: Rc<RefCell<Variable>>, branch: usize) {
        let mut vv = v.borrow_mut();
        let median = vv.get_median();
        if branch == (self.reverse as usize) {
            vv.set_ub(median);
        } else {
            vv.set_lb(median + 1);
        }
    }
}
