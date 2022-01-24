use crate::domain::{Domain, SmallDomain};
use crate::events::{Event, event_index, N_EVENTS};
use crate::propagator::Propagator;
use crate::solver::SolverState;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Variable {
    pub domain: Box<dyn Domain>,
    pub listeners: [HashMap<usize, Rc<RefCell<dyn Propagator>>>; N_EVENTS],
    pub solver_state: Rc<RefCell<SolverState>>,
    pub name: String,
}

impl Variable {
    pub fn new(solver_state: Rc<RefCell<SolverState>>, lb: i64, ub: i64, name: String) -> Self {
        let domain = match ub - lb <= 63 {
            true => Box::new(SmallDomain::new(solver_state.clone(), lb, ub)),
            false => unimplemented!(),
        };
        Self {
            domain,
            listeners: Default::default(),
            solver_state,
            name,
        }
    }
    pub fn assign(&mut self, x: i64) {
        self.notify_listeners(Event::Assigned);
        self.notify_listeners(Event::Modified);
        self.domain.assign(x);
    }
    pub fn is_assigned(&self) -> bool {
        self.domain.is_assigned()
    }
    pub fn fail(&self) {
        self.solver_state.borrow_mut().fail();
    }
    pub fn remove(&mut self, x: i64) {
        self.notify_listeners(Event::Removed);
        self.notify_listeners(Event::Modified);
        self.domain.remove(x);
    }
    pub fn get_lb(&self) -> i64 {
        self.domain.get_lb()
    }
    pub fn get_ub(&self) -> i64 {
        self.domain.get_ub()
    }
    pub fn set_lb(&mut self, x: i64) {
        self.notify_listeners(Event::LowerBound);
        self.notify_listeners(Event::Modified);
        self.domain.set_lb(x)
    }
    pub fn set_ub(&mut self, x: i64) {
        self.notify_listeners(Event::UpperBound);
        self.notify_listeners(Event::Modified);
        self.domain.set_ub(x)
    }
    pub fn value(&self) -> i64 {
        let lb = self.domain.get_lb();
        let ub = self.domain.get_ub();
        if lb != ub {
            panic!("attempted to get value of unassigned variable");
        } else {
            return lb;
        }
    }
    pub fn add_listener(&mut self, listener: Rc<RefCell<dyn Propagator>>, event: Event) {
        let id = event_index(&event);
        let list_id = listener.borrow().get_id();
        self.listeners[id].insert(list_id, listener);
    }
    pub fn notify_listeners(&mut self, event: Event) {
        for (_, listener) in self.listeners[event_index(&event)].drain() {
            if let Ok(mut ref_mut) = listener.try_borrow_mut() {
                ref_mut.new_event();
            } else { // we are inside listener's propagate()
                self.solver_state.borrow_mut().reschedule();
                continue;
            }
            if !listener.borrow().is_queued() {
                listener.borrow_mut().enqueue();
                self.solver_state.borrow_mut().enqueue(listener);
            }
        }
    }
    pub fn rollback(&mut self) {
        self.domain.rollback();
    }
    pub fn checkpoint(&mut self) {
        self.domain.checkpoint();
    }
    pub fn iter(&self) -> impl Iterator<Item = i64> {
        self.domain.iter()
    }
}
