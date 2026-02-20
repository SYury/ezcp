use crate::config::Config;
use crate::constraint::Constraint;
use crate::objective_function::ObjectiveFunction;
use crate::search::{Search, SearchState};
use crate::variable::Variable;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(PartialEq, Eq)]
pub enum SolutionStatus {
    Infeasible,
    Feasible,
    Optimal(i64),
}

pub struct Solver {
    constraints: Vec<Box<dyn Constraint>>,
    variables: Vec<Rc<RefCell<Variable>>>,
    vars_by_name: HashMap<String, Rc<RefCell<Variable>>>,
    const_vars: HashMap<i64, Rc<RefCell<Variable>>>,
    objective: Option<Box<dyn ObjectiveFunction>>,
    state: Rc<RefCell<SearchState>>,
}

impl Solver {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            variables: Vec::new(),
            vars_by_name: HashMap::new(),
            const_vars: HashMap::new(),
            objective: None,
            state: Rc::new(RefCell::new(SearchState::default())),
        }
    }
    pub fn add_constraint(&mut self, c: Box<dyn Constraint>) -> &mut dyn Constraint {
        self.constraints.push(c);
        let r = self.constraints.last_mut().unwrap().as_mut();
        r
    }
    pub fn add_objective(&mut self, objective: Box<dyn ObjectiveFunction>) {
        self.objective = Some(objective);
    }
    /// returns a variable corresponding to the given constant
    pub fn const_variable(&mut self, val: i64) -> Rc<RefCell<Variable>> {
        if let Some(var) = self.const_vars.get(&val) {
            return var.clone();
        }
        let var = Rc::new(RefCell::new(Variable::new(
            self.state.clone(),
            val,
            val,
            format!("_ezcp_internal_const_{}", val),
        )));
        self.variables.push(var.clone());
        var
    }
    /// creates a new variable or returns an existing variable if a variable with the same name exists
    pub fn new_variable(&mut self, lb: i64, ub: i64, name: String) -> Rc<RefCell<Variable>> {
        if let Some(var) = self.vars_by_name.get(&name) {
            return var.clone();
        }
        self.new_var_inner(lb, ub, name)
    }
    /// creates a new variable or returns an existing variable if a variable with the same name exists
    /// if a variable with the same name exists, checks for the same lb/ub
    pub fn new_variable_strict(
        &mut self,
        lb: i64,
        ub: i64,
        name: String,
    ) -> Option<Rc<RefCell<Variable>>> {
        if let Some(var) = self.vars_by_name.get(&name) {
            if var.borrow().get_lb() != lb || var.borrow().get_ub() != ub {
                return None;
            }
            return Some(var.clone());
        }
        Some(self.new_var_inner(lb, ub, name))
    }
    /// creates a new variable, additionally replacing an existing variable if a variable with the same name exists
    /// WARNING: replacing variables used in existing constraints is a very bad idea
    pub fn new_variable_with_replace(
        &mut self,
        lb: i64,
        ub: i64,
        name: String,
    ) -> Rc<RefCell<Variable>> {
        self.new_var_inner(lb, ub, name)
    }
    fn new_var_inner(&mut self, lb: i64, ub: i64, name: String) -> Rc<RefCell<Variable>> {
        let var = Rc::new(RefCell::new(Variable::new(
            self.state.clone(),
            lb,
            ub,
            name.clone(),
        )));
        self.variables.push(var.clone());
        self.vars_by_name.insert(name, var.clone());
        var
    }
    pub fn has_variable(&self, name: &str) -> bool {
        self.vars_by_name.contains_key(name)
    }
    pub fn get_variable_by_name(&self, name: &str) -> Option<Rc<RefCell<Variable>>> {
        self.vars_by_name.get(name).cloned()
    }

    /// Creates new search if no previous search exists.
    pub fn search(&self, config: Config) -> Option<Search<'_>> {
        if self.state.borrow().running {
            None
        } else {
            for v in &self.variables {
                v.borrow_mut().rollback_all();
            }
            Some(Search::new(config, &self.constraints, &self.variables, self.objective.as_ref(), self.state.clone()))
        }
    }
}

/// this function transforms satisfaction problem to minimization problem via binary search
/// create_solver is a function that creates a solver for problem "there is a solution with value <= x"
/// l and r are bounds on optimal solution
/// l < opt
/// r >= opt
pub fn binary_search_optimizer(
    create_solver: impl Fn(i64) -> (Solver, Config),
    mut l: i64,
    mut r: i64,
) -> i64 {
    while r - l > 1 {
        let mid = (l + r) / 2;
        let (solver, config) = create_solver(mid);
        if solver.search(config).unwrap().next().is_some() {
            r = mid;
        } else {
            l = mid;
        }
    }
    r
}
