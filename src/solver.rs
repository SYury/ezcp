use crate::constraint::Constraint;
use crate::propagator::Propagator;
use crate::value_selector::ValueSelector;
use crate::variable::Variable;
use crate::variable_selector::VariableSelector;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::VecDeque;
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

pub struct Solver {
    constraints: Vec<Box<dyn Constraint>>,
    propagators: Vec<Rc<RefCell<dyn Propagator>>>,
    variables: Vec<Rc<RefCell<Variable>>>,
    variable_selector: Box<dyn VariableSelector>,
    value_selector: Box<dyn ValueSelector>,
    state: Rc<RefCell<SolverState>>,
    propagator_id_ctr: usize,
}

impl Solver {
    pub fn new(
        variable_selector: Box<dyn VariableSelector>,
        value_selector: Box<dyn ValueSelector>,
    ) -> Self {
        Self {
            constraints: Vec::new(),
            propagators: Vec::new(),
            variables: Vec::new(),
            variable_selector,
            value_selector,
            state: Rc::new(RefCell::new(SolverState::new())),
            propagator_id_ctr: 0,
        }
    }
    pub fn add_constraint(&mut self, c: Box<dyn Constraint>) -> &mut dyn Constraint {
        c.create_propagators(self);
        self.constraints.push(c);
        let r = self.constraints.last_mut().unwrap().as_mut();
        r
    }
    pub fn add_propagator(&mut self, p: Rc<RefCell<dyn Propagator>>) {
        self.propagators.push(p);
    }
    pub fn new_propagator_id(&mut self) -> usize {
        let id = self.propagator_id_ctr;
        self.propagator_id_ctr += 1;
        id
    }
    pub fn new_variable(&mut self, lb: i64, ub: i64, name: String) -> Rc<RefCell<Variable>> {
        let var = Rc::new(RefCell::new(Variable::new(
            self.state.clone(),
            lb,
            ub,
            name,
        )));
        self.variables.push(var.clone());
        var
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
            if self.state.borrow().resched_current && !p.borrow().is_idemponent() {
                self.state
                    .borrow_mut()
                    .propagation_queue
                    .push_back(p.clone());
                p.borrow_mut().enqueue();
            }
        }
        true
    }

    pub fn solve(&mut self) -> bool {
        #[cfg(debug_assertions)]
        for v in self.variables.iter() {
            print!("VAR {}", v.borrow().name);
            for val in v.borrow().iter() {
                print!(" {}", val);
            }
            println!("");
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
            return true;
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
            println!("fixed value {} for variable {}", x, i);
        }
        v.borrow_mut().assign(x);
        if self.solve() {
            return true;
        }
        #[cfg(debug_assertions)]
        println!("returned after assignment");
        v.borrow_mut().rollback();
        v.borrow_mut().checkpoint();
        v.borrow_mut().remove(x);
        #[cfg(debug_assertions)]
        {
            let mut i = 0;
            while !Rc::ptr_eq(&self.variables[i], &v) {
                i += 1;
            }
            println!("removed value {} from variable {}", x, i);
        }
        if self.solve() {
            return true;
        }
        #[cfg(debug_assertions)]
        println!("returned after removal");
        v.borrow_mut().rollback();
        for v in &mut self.variables {
            v.borrow_mut().rollback();
        }
        false
    }
}
