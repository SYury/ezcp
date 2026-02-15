use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock};
use crate::solver::Solver;
use crate::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

// result = vars[0] AND vars[1] AND ... AND vars[vars.len() - 1]
pub struct AndConstraint {
    result: Rc<RefCell<Variable>>,
    vars: Vec<Rc<RefCell<Variable>>>,
}

impl AndConstraint {
    pub fn new(result: Rc<RefCell<Variable>>, vars: Vec<Rc<RefCell<Variable>>>) -> Self {
        Self { result, vars }
    }
}

impl Constraint for AndConstraint {
    fn satisfied(&self) -> bool {
        let result = match self.result.borrow().is_assigned() {
            true => self.result.borrow().value(),
            false => {
                return false;
            }
        };
        for v in &self.vars {
            if !v.borrow().is_assigned() {
                return false;
            }
            if v.borrow().value() == 0 {
                return result == 0;
            }
        }
        result != 0
    }

    fn create_propagators(&self, solver: &mut Solver) {
        let p = Rc::new(RefCell::new(AndPropagator::new(
            self.result.clone(),
            self.vars.clone(),
            solver.new_propagator_id(),
        )));
        solver.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

pub struct AndPropagator {
    pcb: PropagatorControlBlock,
    result: Rc<RefCell<Variable>>,
    vars: Vec<Rc<RefCell<Variable>>>,
}

impl AndPropagator {
    pub fn new(result: Rc<RefCell<Variable>>, vars: Vec<Rc<RefCell<Variable>>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            result,
            vars,
        }
    }
}

impl Propagator for AndPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.result
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        for v in &self.vars {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(&mut self) {
        if self.result.borrow().is_assigned() {
            let result = self.result.borrow().value();
            if result == 1 {
                for v in &self.vars {
                    v.borrow_mut().assign(1);
                }
            } else {
                let mut ones = 0;
                let mut unknown = 0;
                for v in &self.vars {
                    if v.borrow().is_assigned() {
                        if v.borrow().value() == 1 {
                            ones += 1;
                        }
                    } else {
                        unknown += 1;
                    }
                }
                if ones == self.vars.len() {
                    self.result.borrow_mut().fail();
                } else if unknown == 1 && 1 + ones == self.vars.len() {
                    for v in &self.vars {
                        if !v.borrow().is_assigned() {
                            v.borrow_mut().assign(0);
                        }
                    }
                }
            }
        } else {
            let mut can0 = false;
            let mut can1 = true;
            for v in &self.vars {
                if v.borrow().possible(0) {
                    can0 = true;
                }
                if !v.borrow().possible(1) {
                    can1 = false;
                }
            }
            if !can0 {
                self.result.borrow_mut().remove(0);
            }
            if !can1 {
                self.result.borrow_mut().remove(1);
            }
        }
    }

    fn get_cb(&self) -> &PropagatorControlBlock {
        &self.pcb
    }

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock {
        &mut self.pcb
    }

    fn is_idemponent(&self) -> bool {
        true
    }
}

// result = vars[0] OR vars[1] OR ... OR vars[vars.len() - 1]
pub struct OrConstraint {
    result: Rc<RefCell<Variable>>,
    vars: Vec<Rc<RefCell<Variable>>>,
}

impl OrConstraint {
    pub fn new(result: Rc<RefCell<Variable>>, vars: Vec<Rc<RefCell<Variable>>>) -> Self {
        Self { result, vars }
    }
}

impl Constraint for OrConstraint {
    fn satisfied(&self) -> bool {
        let result = match self.result.borrow().is_assigned() {
            true => self.result.borrow().value(),
            false => {
                return false;
            }
        };
        for v in &self.vars {
            if !v.borrow().is_assigned() {
                return false;
            }
            if v.borrow().value() == 1 {
                return result != 0;
            }
        }
        result == 0
    }

    fn create_propagators(&self, solver: &mut Solver) {
        let p = Rc::new(RefCell::new(OrPropagator::new(
            self.result.clone(),
            self.vars.clone(),
            solver.new_propagator_id(),
        )));
        solver.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

pub struct OrPropagator {
    pcb: PropagatorControlBlock,
    result: Rc<RefCell<Variable>>,
    vars: Vec<Rc<RefCell<Variable>>>,
}

impl OrPropagator {
    pub fn new(result: Rc<RefCell<Variable>>, vars: Vec<Rc<RefCell<Variable>>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            result,
            vars,
        }
    }
}

impl Propagator for OrPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.result
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        for v in &self.vars {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(&mut self) {
        if self.result.borrow().is_assigned() {
            let result = self.result.borrow().value();
            if result == 1 {
                let mut ones = 0;
                for v in &self.vars {
                    if v.borrow().possible(1) {
                        ones += 1;
                    }
                }
                if ones == 0 {
                    self.result.borrow_mut().fail();
                    return;
                }
                if ones == 1 {
                    for v in &self.vars {
                        if v.borrow().possible(1) {
                            v.borrow_mut().assign(1);
                        }
                    }
                }
            } else {
                for v in &self.vars {
                    v.borrow_mut().assign(0);
                }
            }
        } else {
            let mut can1 = false;
            let mut can0 = true;
            for v in &self.vars {
                if v.borrow().possible(1) {
                    can1 = true;
                }
                if !v.borrow().possible(0) {
                    can0 = false;
                }
            }
            if !can0 {
                self.result.borrow_mut().remove(0);
            }
            if !can1 {
                self.result.borrow_mut().remove(1);
            }
        }
    }

    fn get_cb(&self) -> &PropagatorControlBlock {
        &self.pcb
    }

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock {
        &mut self.pcb
    }

    fn is_idemponent(&self) -> bool {
        true
    }
}

// x = not y
pub struct NegateConstraint {
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
}

impl NegateConstraint {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>) -> Self {
        Self { x, y }
    }
}

impl Constraint for NegateConstraint {
    fn satisfied(&self) -> bool {
        if !self.x.borrow().is_assigned() || !self.y.borrow().is_assigned() {
            false
        } else {
            self.x.borrow().value() != self.y.borrow().value()
        }
    }

    fn create_propagators(&self, solver: &mut Solver) {
        let p = Rc::new(RefCell::new(NegatePropagator::new(
            self.x.clone(),
            self.y.clone(),
            solver.new_propagator_id(),
        )));
        solver.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

pub struct NegatePropagator {
    pcb: PropagatorControlBlock,
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
}

impl NegatePropagator {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            x,
            y,
        }
    }
}

impl Propagator for NegatePropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.x
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        self.y
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
    }

    fn propagate(&mut self) {
        for val in 0..2 {
            if !self.x.borrow().possible(val) {
                self.y.borrow_mut().remove(val ^ 1);
            }
        }
        for val in 0..2 {
            if !self.y.borrow().possible(val) {
                self.x.borrow_mut().remove(val ^ 1);
            }
        }
    }

    fn get_cb(&self) -> &PropagatorControlBlock {
        &self.pcb
    }

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock {
        &mut self.pcb
    }

    fn is_idemponent(&self) -> bool {
        true
    }
}
