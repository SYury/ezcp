use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock, PropagatorState};
use crate::search::Search;
use crate::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

pub struct EqConstraint {
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
}

impl EqConstraint {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>) -> Self {
        Self { x, y }
    }
}

impl Constraint for EqConstraint {
    fn satisfied(&self) -> bool {
        self.x.borrow().is_assigned()
            && self.y.borrow().is_assigned()
            && self.x.borrow().value() == self.y.borrow().value()
    }

    fn failed(&self) -> bool {
        self.x.borrow().get_ub() < self.y.borrow().get_lb()
            || self.y.borrow().get_ub() < self.x.borrow().get_lb()
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(EqBCPropagator::new(
            self.x.clone(),
            self.y.clone(),
            index0,
        )))]
    }
}

pub struct EqBCPropagator {
    pcb: PropagatorControlBlock,
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
}

impl EqBCPropagator {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            x,
            y,
        }
    }
}

impl Propagator for EqBCPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.x
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::LowerBound);
        self.x
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::UpperBound);
        self.y
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::LowerBound);
        self.y
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::UpperBound);
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.x
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::LowerBound);
        self.x
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::UpperBound);
        self.y
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::LowerBound);
        self.y
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::UpperBound);
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
        let x = self.x.borrow();
        let y = self.y.borrow();
        let l = x.get_lb().max(y.get_lb());
        let u = x.get_ub().min(y.get_ub());
        drop(x);
        drop(y);
        let mut x = self.x.borrow_mut();
        x.set_lb(l);
        x.set_ub(u);
        drop(x);
        let mut y = self.y.borrow_mut();
        y.set_lb(l);
        y.set_ub(u);
        PropagatorState::Normal
    }

    fn get_cb(&self) -> &PropagatorControlBlock {
        &self.pcb
    }

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock {
        &mut self.pcb
    }

    fn is_idempotent(&self) -> bool {
        true
    }
}

pub struct NeqConstraint {
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
}

impl NeqConstraint {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>) -> Self {
        Self { x, y }
    }
}

impl Constraint for NeqConstraint {
    fn satisfied(&self) -> bool {
        self.x.borrow().is_assigned()
            && self.y.borrow().is_assigned()
            && self.x.borrow().value() != self.y.borrow().value()
    }

    fn failed(&self) -> bool {
        self.x.borrow().is_assigned()
            && self.y.borrow().is_assigned()
            && self.x.borrow().value() == self.y.borrow().value()
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(NeqPropagator::new(
            self.x.clone(),
            self.y.clone(),
            index0,
        )))]
    }
}

pub struct NeqPropagator {
    pcb: PropagatorControlBlock,
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
}

impl NeqPropagator {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            x,
            y,
        }
    }
}

impl Propagator for NeqPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.x
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Assigned);
        self.y
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Assigned);
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.x
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::Assigned);
        self.y
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::Assigned);
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
        if let Some(v) = self.x.borrow().try_value() {
            self.y.borrow_mut().remove(v);
        } else if let Some(v) = self.y.borrow().try_value() {
            self.x.borrow_mut().remove(v);
        }
        PropagatorState::Terminated
    }

    fn get_cb(&self) -> &PropagatorControlBlock {
        &self.pcb
    }

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock {
        &mut self.pcb
    }

    fn is_idempotent(&self) -> bool {
        true
    }
}
