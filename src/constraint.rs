use crate::solver::Solver;
use std::cell::RefCell;
use std::rc::Rc;
use crate::variable::Variable;

pub trait Constraint {
    fn satisfied(&self) -> bool;
    /// this function is run whenever the constraint is added to solver
    fn create_propagators(&self, solver: &mut Solver);
}
