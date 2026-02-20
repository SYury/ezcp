use crate::brancher::{Brancher, MinValueBrancher};
use crate::variable_selector::{FirstFailVariableSelector, VariableSelector};

pub struct Config {
    pub brancher: Box<dyn Brancher>,
    pub variable_selector: Box<dyn VariableSelector>,
}

impl Config {
    pub fn new(brancher: Box<dyn Brancher>, variable_selector: Box<dyn VariableSelector>) -> Self {
        Self {
            brancher,
            variable_selector,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            brancher: Box::new(MinValueBrancher {}),
            variable_selector: Box::new(FirstFailVariableSelector {}),
        }
    }
}
