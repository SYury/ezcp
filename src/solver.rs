use crate::config::Config;
use crate::constraint::Constraint;
use crate::objective_function::ObjectiveFunction;
use crate::propagator::Propagator;
use crate::value_selector::ValueSelector;
use crate::variable::Variable;
use crate::variable_selector::VariableSelector;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

pub struct SolverState {
    status: i32,
    propagation_queue: VecDeque<Rc<RefCell<dyn Propagator>>>,
    resched_current: bool,
}

impl SolverState {
    pub fn new() -> Self {
        Self {
            status: 0,
            propagation_queue: VecDeque::new(),
            resched_current: false,
        }
    }
    pub fn fail(&mut self) {
        self.status = -1
    }
    pub fn enqueue(&mut self, listener: Rc<RefCell<dyn Propagator>>) {
        self.propagation_queue.push_back(listener);
    }
    pub fn reschedule(&mut self) {
        self.resched_current = true;
    }
}

#[derive(PartialEq, Eq)]
pub enum SolutionStatus {
    Infeasible,
    Feasible,
    Optimal(i64),
}

pub struct Solver {
    constraints: Vec<Box<dyn Constraint>>,
    propagators: Vec<Rc<RefCell<dyn Propagator>>>,
    variables: Vec<Rc<RefCell<Variable>>>,
    vars_by_name: HashMap<String, Rc<RefCell<Variable>>>,
    variable_selector: Box<dyn VariableSelector>,
    value_selector: Box<dyn ValueSelector>,
    state: Rc<RefCell<SolverState>>,
    objective: Option<Box<dyn ObjectiveFunction>>,
    current_min: i64,
    best_solution: Vec<i64>,
    propagator_id_ctr: usize,
}

impl Solver {
    pub fn new(config: Config) -> Self {
        Self {
            constraints: Vec::new(),
            propagators: Vec::new(),
            variables: Vec::new(),
            vars_by_name: HashMap::new(),
            variable_selector: config.variable_selector,
            value_selector: config.value_selector,
            state: Rc::new(RefCell::new(SolverState::new())),
            objective: None,
            current_min: i64::MAX,
            best_solution: Vec::new(),
            propagator_id_ctr: 0,
        }
    }
    pub fn add_constraint(&mut self, c: Box<dyn Constraint>) -> &mut dyn Constraint {
        c.create_propagators(self);
        self.constraints.push(c);
        let r = self.constraints.last_mut().unwrap().as_mut();
        r
    }
    pub fn add_objective(&mut self, objective: Box<dyn ObjectiveFunction>) {
        self.objective = Some(objective);
    }
    pub fn add_propagator(&mut self, p: Rc<RefCell<dyn Propagator>>) {
        self.propagators.push(p);
    }
    pub fn get_objective(&self) -> i64 {
        self.current_min
    }
    pub fn new_propagator_id(&mut self) -> usize {
        let id = self.propagator_id_ctr;
        self.propagator_id_ctr += 1;
        id
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
    pub fn get_variable_by_name(&self, name: &str) -> Option<Rc<RefCell<Variable>>> {
        self.vars_by_name.get(name).cloned()
    }
    pub fn check_solution(&self) -> bool {
        for c in &self.constraints {
            if !c.satisfied() {
                return false;
            }
        }
        true
    }

    pub fn propagate(&mut self) -> bool {
        while !self.state.borrow().propagation_queue.is_empty() {
            self.state.borrow_mut().resched_current = false;
            let p = self
                .state
                .borrow_mut()
                .propagation_queue
                .pop_front()
                .unwrap();
            p.borrow_mut().dequeue();
            p.borrow_mut().clear_events();
            p.borrow_mut().propagate();
            p.borrow().listen(p.clone());
            if self.state.borrow().status == -1 {
                for prop in self.state.borrow_mut().propagation_queue.drain(..) {
                    prop.borrow_mut().dequeue();
                    prop.borrow().listen(prop.clone());
                }
                return false;
            }
            if self.state.borrow().resched_current && !p.borrow().is_idempotent() {
                self.state
                    .borrow_mut()
                    .propagation_queue
                    .push_back(p.clone());
                p.borrow_mut().enqueue();
            }
        }
        true
    }

    fn search(&mut self) -> bool {
        #[cfg(debug_assertions)]
        if self.objective.is_some() {
            eprintln!("current best objective = {}", self.current_min);
        }
        #[cfg(debug_assertions)]
        for v in self.variables.iter() {
            eprint!("VAR {}", v.borrow().name);
            for val in v.borrow().iter() {
                eprint!(" {}", val);
            }
            eprintln!();
        }
        for v in &mut self.variables {
            v.borrow_mut().checkpoint();
        }
        if !self.propagate() {
            for v in &mut self.variables {
                v.borrow_mut().rollback();
            }
            self.state.borrow_mut().status = 0;
            return false;
        }
        let mut vars = Vec::new();
        for v in &self.variables {
            if !v.borrow().is_assigned() {
                vars.push(v.clone());
            }
        }
        if vars.is_empty() {
            if !self.check_solution() {
                for v in &mut self.variables {
                    v.borrow_mut().rollback();
                }
                return false;
            }
            if let Some(objective) = &self.objective {
                let val = objective.eval();
                if val < self.current_min {
                    self.current_min = val;
                    if self.best_solution.is_empty() {
                        self.best_solution = vec![0i64; self.variables.len()];
                    }
                    for (i, var) in self.variables.iter().enumerate() {
                        self.best_solution[i] = var.borrow().value();
                    }
                }
                for v in &mut self.variables {
                    v.borrow_mut().rollback();
                }
            }
            return true;
        }
        if let Some(objective) = &self.objective {
            let bound = objective.bound();
            if bound >= self.current_min {
                for v in &mut self.variables {
                    v.borrow_mut().rollback();
                }
                return false;
            }
        }
        let v = self.variable_selector.select(vars);
        let x = self.value_selector.select(v.borrow().domain.as_ref());
        v.borrow_mut().checkpoint();
        #[cfg(debug_assertions)]
        {
            let mut i = 0;
            while !Rc::ptr_eq(&self.variables[i], &v) {
                i += 1;
            }
            eprintln!("fixed value {} for variable {}", x, i);
        }
        v.borrow_mut().assign(x);
        let mut found = false;
        if self.search() {
            if self.objective.is_none() {
                return true;
            } else {
                found = true;
            }
        }
        #[cfg(debug_assertions)]
        eprintln!("returned after assignment");
        v.borrow_mut().rollback();
        v.borrow_mut().checkpoint();
        v.borrow_mut().remove(x);
        #[cfg(debug_assertions)]
        {
            let mut i = 0;
            while !Rc::ptr_eq(&self.variables[i], &v) {
                i += 1;
            }
            eprintln!("removed value {} from variable {}", x, i);
        }
        if self.search() {
            if self.objective.is_none() {
                return true;
            } else {
                found = true;
            }
        }
        #[cfg(debug_assertions)]
        eprintln!("returned after removal");
        v.borrow_mut().rollback();
        for v in &mut self.variables {
            v.borrow_mut().rollback();
        }
        found
    }

    pub fn solve(&mut self) -> SolutionStatus {
        let res = self.search();
        if self.objective.is_some() && res {
            for (i, v) in self.variables.iter_mut().enumerate() {
                v.borrow_mut().assign(self.best_solution[i]);
            }
            assert!(self.check_solution());
            return SolutionStatus::Optimal(self.objective.as_ref().unwrap().eval());
        }
        if res {
            assert!(self.check_solution());
            SolutionStatus::Feasible
        } else {
            SolutionStatus::Infeasible
        }
    }
}

// this function transforms satisfaction problem to minimization problem via binary search
// create_solver is a function that creates a solver for problem "there is a solution with value <= x"
// l and r are bounds on optimal solution
// l < opt
// r >= opt
pub fn binary_search_optimizer(
    create_solver: impl Fn(i64) -> Solver,
    mut l: i64,
    mut r: i64,
) -> i64 {
    while r - l > 1 {
        let mid = (l + r) / 2;
        let mut solver = create_solver(mid);
        if solver.solve() != SolutionStatus::Infeasible {
            r = mid;
        } else {
            l = mid;
        }
    }
    r
}
