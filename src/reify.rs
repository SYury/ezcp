use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock, PropagatorState};
use crate::search::Search;
use crate::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

/// (b = 1) => C is satisfied (half-reification)
pub struct ImpliedConstraint {
    pub b: Rc<RefCell<Variable>>,
    pub c: Rc<RefCell<dyn Constraint>>,
}

impl ImpliedConstraint {
    pub fn new(b: Rc<RefCell<Variable>>, c: Rc<RefCell<dyn Constraint>>) -> Self {
        Self { b, c }
    }
}

impl Constraint for ImpliedConstraint {
    fn satisfied(&self) -> bool {
        let b = self.b.borrow();
        if !b.is_assigned() {
            return false;
        }
        if b.value() == 0 {
            return true;
        }
        self.c.borrow().satisfied()
    }

    fn failed(&self) -> bool {
        let b = self.b.borrow();
        if !b.is_assigned() {
            return false;
        }
        if b.value() == 0 {
            return false;
        }
        self.c.borrow().failed()
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(ImpliedPropagator {
            pcb: PropagatorControlBlock::new(index0),
            b: self.b.clone(),
            c: self.c.clone(),
        }))]
    }
}

pub struct ImpliedPropagator {
    pcb: PropagatorControlBlock,
    b: Rc<RefCell<Variable>>,
    c: Rc<RefCell<dyn Constraint>>,
}

impl Propagator for ImpliedPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.b
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        let prop = self.c.borrow_mut().create_propagators(0);
        for p in prop {
            p.borrow().listen(self_pointer.clone());
        }
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.b
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::Modified);
        let prop = self.c.borrow_mut().create_propagators(0);
        for p in prop {
            p.borrow().unlisten(self_pointer.clone());
        }
    }

    fn propagate(
        &mut self,
        _self_pointer: Rc<RefCell<dyn Propagator>>,
        search: &mut Search<'_>,
    ) -> PropagatorState {
        if self.b.borrow().is_assigned() {
            if self.b.borrow().value() == 1 {
                self.c.borrow().add_propagators(search);
            }
            PropagatorState::Terminated
        } else {
            if self.c.borrow().failed() {
                self.b.borrow_mut().assign(0);
            }
            PropagatorState::Normal
        }
    }

    fn get_cb(&self) -> &PropagatorControlBlock {
        &self.pcb
    }

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock {
        &mut self.pcb
    }

    fn is_idempotent(&self) -> bool {
        false
    }
}

/// (b = 1) <=> C is satisfied
pub struct ReifiedConstraint {
    pub b: Rc<RefCell<Variable>>,
    pub c: Rc<RefCell<dyn Constraint>>,
    pub notc: Rc<RefCell<dyn Constraint>>,
}

impl ReifiedConstraint {
    pub fn new(
        b: Rc<RefCell<Variable>>,
        c: Rc<RefCell<dyn Constraint>>,
        notc: Rc<RefCell<dyn Constraint>>,
    ) -> Self {
        Self { b, c, notc }
    }
}

impl Constraint for ReifiedConstraint {
    fn satisfied(&self) -> bool {
        let b = self.b.borrow();
        if !b.is_assigned() {
            return false;
        }
        if b.value() == 0 {
            self.notc.borrow().satisfied()
        } else {
            self.c.borrow().satisfied()
        }
    }

    fn failed(&self) -> bool {
        let b = self.b.borrow();
        if !b.is_assigned() {
            return false;
        }
        if b.value() == 0 {
            self.notc.borrow().failed()
        } else {
            self.c.borrow().failed()
        }
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(ReifiedPropagator {
            pcb: PropagatorControlBlock::new(index0),
            b: self.b.clone(),
            c: self.c.clone(),
            notc: self.notc.clone(),
        }))]
    }
}

pub struct ReifiedPropagator {
    pcb: PropagatorControlBlock,
    b: Rc<RefCell<Variable>>,
    c: Rc<RefCell<dyn Constraint>>,
    notc: Rc<RefCell<dyn Constraint>>,
}

impl Propagator for ReifiedPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.b
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        let prop = self.c.borrow().create_propagators(0);
        for p in prop {
            p.borrow().listen(self_pointer.clone());
        }
        let prop1 = self.notc.borrow().create_propagators(0);
        for p in prop1 {
            p.borrow().listen(self_pointer.clone());
        }
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.b
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::Modified);
        let prop = self.c.borrow().create_propagators(0);
        for p in prop {
            p.borrow().unlisten(self_pointer.clone());
        }
        let prop1 = self.notc.borrow().create_propagators(0);
        for p in prop1 {
            p.borrow().unlisten(self_pointer.clone());
        }
    }

    fn propagate(
        &mut self,
        _self_pointer: Rc<RefCell<dyn Propagator>>,
        search: &mut Search<'_>,
    ) -> PropagatorState {
        if self.b.borrow().is_assigned() {
            if self.b.borrow().value() == 1 {
                self.c.borrow().add_propagators(search);
            } else {
                self.notc.borrow().add_propagators(search);
            }
            PropagatorState::Terminated
        } else {
            if self.c.borrow().failed() {
                self.b.borrow_mut().assign(0);
            }
            if self.notc.borrow().failed() {
                self.b.borrow_mut().assign(1);
            }
            PropagatorState::Normal
        }
    }

    fn get_cb(&self) -> &PropagatorControlBlock {
        &self.pcb
    }

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock {
        &mut self.pcb
    }

    fn is_idempotent(&self) -> bool {
        false
    }
}
