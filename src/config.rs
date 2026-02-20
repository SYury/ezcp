use crate::brancher::{Brancher, MinValueBrancher};
use crate::variable_selector::{FirstFailVariableSelector, VariableSelector};

pub struct Config {
    pub brancher: Box<dyn Brancher>,
    pub variable_selector: Box<dyn VariableSelector>,
    /// For constraint satisfaction problems (no objective function) the search will return all feasible solutions.
    /// For constraint optimization problems the search will return the sequence of objective-improving solutions.
    pub all_solutions: bool,
}

impl Config {
    pub fn new(brancher: Box<dyn Brancher>, variable_selector: Box<dyn VariableSelector>, all_solutions: bool) -> Self {
        Self {
            brancher,
            variable_selector,
            all_solutions,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            brancher: Box::new(MinValueBrancher {}),
            variable_selector: Box::new(FirstFailVariableSelector {}),
            all_solutions: false,
        }
    }
}
