use crate::value_selector::{MinValueSelector, ValueSelector};
use crate::variable_selector::{FirstFailVariableSelector, VariableSelector};

pub struct Config {
    pub value_selector: Box<dyn ValueSelector>,
    pub variable_selector: Box<dyn VariableSelector>,
}

impl Config {
    pub fn new(
        value_selector: Box<dyn ValueSelector>,
        variable_selector: Box<dyn VariableSelector>,
    ) -> Self {
        Self {
            value_selector,
            variable_selector,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            value_selector: Box::new(MinValueSelector {}),
            variable_selector: Box::new(FirstFailVariableSelector {}),
        }
    }
}
