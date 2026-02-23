use crate::brancher::Brancher;
use crate::config::Config;
use crate::constraint::Constraint;
use crate::objective_function::ObjectiveFunction;
use crate::propagator::{Propagator, PropagatorState};
use crate::variable::Variable;
use crate::variable_selector::VariableSelector;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Default)]
pub struct SearchState {
    status: i32,
    propagation_queue: VecDeque<Rc<RefCell<dyn Propagator>>>,
    resched_current: bool,
    pub running: bool,
}

impl SearchState {
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

#[derive(Clone, Default)]
struct SearchNode {
    pub var: Option<Rc<RefCell<Variable>>>,
    pub branch: usize,
    pub n_branches: usize,
    pub n_propagators: usize,
    pub terminated: Vec<Rc<RefCell<dyn Propagator>>>,
}

pub struct Search<'a> {
    constraints: &'a [Box<dyn Constraint>],
    propagators: Vec<Rc<RefCell<dyn Propagator>>>,
    variables: &'a [Rc<RefCell<Variable>>],
    variable_selector: Box<dyn VariableSelector>,
    brancher: Box<dyn Brancher>,
    branchable_vars: Vec<Rc<RefCell<Variable>>>,
    all_solutions: bool,
    state: Rc<RefCell<SearchState>>,
    objective: Option<&'a dyn ObjectiveFunction>,
    current_min: i64,
    best_solution: Vec<i64>,
    propagator_id_ctr: usize,
    stack: Vec<SearchNode>,
    stats: SearchStats,
}

#[derive(Clone, Default)]
pub struct SearchStats {
    pub depth: usize,
    pub max_depth: usize,
    pub fails: usize,
    pub total_solutions_reported: usize,
}

impl<'a> Search<'a> {
    pub fn new(
        config: Config,
        constraints: &'a [Box<dyn Constraint>],
        variables: &'a [Rc<RefCell<Variable>>],
        objective: Option<&'a dyn ObjectiveFunction>,
        state: Rc<RefCell<SearchState>>,
    ) -> Self {
        {
            let mut s = state.borrow_mut();
            *s = SearchState::default();
            s.running = true;
        }
        let mut search = Self {
            constraints,
            variables,
            propagators: Vec::new(),
            variable_selector: config.variable_selector,
            brancher: config.brancher,
            branchable_vars: config.branchable_vars,
            all_solutions: config.all_solutions,
            state,
            objective,
            current_min: i64::MAX,
            best_solution: Vec::new(),
            propagator_id_ctr: 0,
            stack: vec![SearchNode::default()],
            stats: SearchStats::default(),
        };
        for c in constraints {
            c.add_propagators(&mut search);
        }
        search
    }

    pub fn add_propagator(&mut self, p: Rc<RefCell<dyn Propagator>>) {
        self.propagators.push(p);
    }

    pub fn get_propagator_id(&self) -> usize {
        self.propagator_id_ctr
    }

    pub fn advance_propagator_id(&mut self, x: usize) {
        self.propagator_id_ctr += x;
    }

    pub fn get_objective(&self) -> i64 {
        self.current_min
    }

    pub fn check_solution(&self) -> bool {
        for c in self.constraints {
            if !c.satisfied() {
                return false;
            }
        }
        true
    }

    pub fn propagate(&mut self, terminated: &mut Vec<Rc<RefCell<dyn Propagator>>>) -> bool {
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
            let state = p.borrow_mut().propagate(p.clone(), self);
            match state {
                PropagatorState::Normal => {
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
                PropagatorState::Terminated => {
                    if self.state.borrow().status == -1 {
                        p.borrow().listen(p.clone());
                        for prop in self.state.borrow_mut().propagation_queue.drain(..) {
                            prop.borrow_mut().dequeue();
                            prop.borrow().listen(prop.clone());
                        }
                        return false;
                    }
                    p.borrow().unlisten(p.clone());
                    terminated.push(p);
                }
            }
        }
        true
    }

    fn restore_propagators(&mut self, node: &SearchNode) {
        for p in &node.terminated {
            p.borrow().listen(p.clone());
        }
        for p in &self.propagators[node.n_propagators..] {
            p.borrow().unlisten(p.clone());
        }
        self.propagators.truncate(node.n_propagators);
    }
}

impl Iterator for Search<'_> {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stats.total_solutions_reported > 0 {
            if !self.all_solutions {
                return None;
            }
            for v in self.variables {
                v.borrow_mut().rollback();
            }
        }
        while let Some(mut node) = self.stack.last().cloned() {
            if let Some(var) = &node.var {
                if node.branch > 0 {
                    #[cfg(debug_assertions)]
                    {
                        eprintln!("returned from branch {}", node.branch - 1);
                    }
                    var.borrow_mut().rollback();
                }
                if node.branch == node.n_branches {
                    self.restore_propagators(&node);
                    for v in self.variables {
                        v.borrow_mut().rollback();
                    }
                    self.stack.pop();
                    continue;
                }
                var.borrow_mut().checkpoint();
                self.brancher.branch(var.clone(), node.branch);
                #[cfg(debug_assertions)]
                {
                    eprintln!(
                        "branch #{} for variable {}",
                        node.branch,
                        &var.borrow().name
                    );
                }
                self.stack.last_mut().unwrap().branch += 1;
                self.stack.push(SearchNode::default());
                continue;
            } else {
                #[cfg(debug_assertions)]
                if self.objective.is_some() {
                    eprintln!(
                        "entered new search node; current best objective = {}",
                        self.current_min
                    );
                }
                #[cfg(debug_assertions)]
                for v in self.variables {
                    eprint!(
                        "VAR {} in [{}, {}]; DOM = ",
                        v.borrow().name,
                        v.borrow().get_lb(),
                        v.borrow().get_ub()
                    );
                    for val in v.borrow().iter() {
                        eprint!(" {}", val);
                    }
                    eprintln!();
                }
                for v in self.variables {
                    v.borrow_mut().checkpoint();
                }
                assert!(node.terminated.is_empty());
                node.n_propagators = self.propagators.len();
                if !self.propagate(&mut node.terminated) {
                    for v in self.variables {
                        v.borrow_mut().rollback();
                    }
                    self.restore_propagators(&node);
                    self.state.borrow_mut().status = 0;
                    self.stack.pop();
                    continue;
                }
                let mut vars = Vec::new();
                if self.branchable_vars.is_empty() {
                    for v in self.variables {
                        if !v.borrow().is_assigned() {
                            vars.push(v.clone());
                        }
                    }
                } else {
                    for v in &self.branchable_vars {
                        if !v.borrow().is_assigned() {
                            vars.push(v.clone());
                        }
                    }
                }
                if vars.is_empty() {
                    if !self.check_solution() {
                        for v in self.variables {
                            v.borrow_mut().rollback();
                        }
                        self.restore_propagators(&node);
                        self.stack.pop();
                        continue;
                    }
                    if let Some(objective) = self.objective {
                        let val = objective.eval();
                        if val < self.current_min {
                            self.current_min = val;
                            if self.best_solution.is_empty() {
                                self.best_solution = vec![0i64; self.variables.len()];
                            }
                            for (i, var) in self.variables.iter().enumerate() {
                                self.best_solution[i] = var.borrow().value();
                            }
                            if self.all_solutions {
                                self.restore_propagators(&node);
                                self.stack.pop();
                                self.stats.total_solutions_reported += 1;
                                return Some(val);
                            }
                        }
                        for v in self.variables {
                            v.borrow_mut().rollback();
                        }
                        self.restore_propagators(&node);
                        self.stack.pop();
                        continue;
                    }
                    self.restore_propagators(&node);
                    self.stack.pop();
                    self.stats.total_solutions_reported += 1;
                    return Some(0);
                }
                #[cfg(debug_assertions)]
                {
                    eprintln!("PROPAGATION AT NON-TRIVIAL FIXPOINT; STATE AFTER PROPAGATION:");
                    for v in self.variables {
                        eprint!(
                            "VAR {} in [{}, {}]; DOM = ",
                            v.borrow().name,
                            v.borrow().get_lb(),
                            v.borrow().get_ub()
                        );
                        for val in v.borrow().iter() {
                            eprint!(" {}", val);
                        }
                        eprintln!();
                    }
                }
                if let Some(objective) = &self.objective {
                    let bound = objective.bound();
                    if bound >= self.current_min {
                        for v in self.variables {
                            v.borrow_mut().rollback();
                        }
                        self.restore_propagators(&node);
                        self.stack.pop();
                        continue;
                    }
                }
                let v = self.variable_selector.select(vars);
                let br = self.brancher.n_branches(v.clone());
                self.stack.pop();
                node.var = Some(v.clone());
                node.branch = 0;
                node.n_branches = br;
                self.stack.push(node);
                continue;
            }
        }
        if !self.all_solutions && !self.best_solution.is_empty() {
            self.stats.total_solutions_reported = 1;
            for (i, v) in self.variables.iter().enumerate() {
                v.borrow_mut().assign(self.best_solution[i]);
            }
            return Some(self.current_min);
        }
        None
    }
}

impl Drop for Search<'_> {
    fn drop(&mut self) {
        self.state.borrow_mut().running = false;
    }
}
