use crate::brancher::{Brancher, MinValueBrancher};
use crate::variable::Variable;
use crate::variable_selector::{FirstFailVariableSelector, VariableSelector};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Config {
    pub brancher: Box<dyn Brancher>,
    pub variable_selector: Box<dyn VariableSelector>,
    /// If this vector is empty, all non-constant variables will be used for branching.
    pub branchable_vars: Vec<Rc<RefCell<Variable>>>,
    /// For constraint satisfaction problems (no objective function) the search will return all feasible solutions.
    /// For constraint optimization problems the search will return the sequence of objective-improving solutions.
    pub all_solutions: bool,
}

impl Config {
    pub fn new(
        brancher: Box<dyn Brancher>,
        variable_selector: Box<dyn VariableSelector>,
        branchable_vars: Vec<Rc<RefCell<Variable>>>,
        all_solutions: bool,
    ) -> Self {
        Self {
            brancher,
            variable_selector,
            branchable_vars,
            all_solutions,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            brancher: Box::new(MinValueBrancher {}),
            variable_selector: Box::new(FirstFailVariableSelector {}),
            branchable_vars: Vec::default(),
            all_solutions: false,
        }
    }
}
