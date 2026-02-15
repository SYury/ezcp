use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock};
use crate::solver::Solver;
use crate::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

// assuming q > 0
fn floor_div(p: i64, q: i64) -> i64 {
    if p > 0 {
        p / q
    } else {
        -((-p + q - 1) / q)
    }
}

// assuming q > 0
fn ceil_div(p: i64, q: i64) -> i64 {
    if p > 0 {
        (p + q - 1) / q
    } else {
        -((-p) / q)
    }
}

// sum x[i] * a[i] <= b
pub struct LinearInequalityConstraint {
    x: Vec<Rc<RefCell<Variable>>>,
    a: Vec<i64>,
    b: i64,
}

impl LinearInequalityConstraint {
    pub fn new(x: Vec<Rc<RefCell<Variable>>>, a: Vec<i64>, b: i64) -> Self {
        assert!(x.len() == a.len());
        Self { x, a, b }
    }
}

impl Constraint for LinearInequalityConstraint {
    fn satisfied(&self) -> bool {
        let mut sum = 0;
        for (x, a) in self.x.iter().zip(self.a.iter()) {
            if !x.borrow().is_assigned() {
                return false;
            }
            sum += x.borrow().value() * (*a);
        }
        sum <= self.b
    }

    fn create_propagators(&self, solver: &mut Solver) {
        let p = Rc::new(RefCell::new(LinearInequalityPropagator::new(
            self.x.clone(),
            self.a.clone(),
            self.b,
            solver.new_propagator_id(),
        )));
        solver.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

pub struct LinearInequalityPropagator {
    pcb: PropagatorControlBlock,
    x: Vec<Rc<RefCell<Variable>>>,
    a: Vec<i64>,
    b: i64,
}

impl LinearInequalityPropagator {
    pub fn new(x: Vec<Rc<RefCell<Variable>>>, a: Vec<i64>, b: i64, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            x,
            a,
            b,
        }
    }
}

impl Propagator for LinearInequalityPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for (i, x) in self.x.iter().enumerate() {
            if self.a[i] > 0 {
                x.borrow_mut()
                    .add_listener(self_pointer.clone(), Event::LowerBound);
            } else {
                x.borrow_mut()
                    .add_listener(self_pointer.clone(), Event::UpperBound);
            }
        }
    }

    fn propagate(&mut self) {
        let mut lower_sum = 0;
        for (xx, a) in self.x.iter().zip(self.a.iter().copied()) {
            let x = xx.borrow();
            if a > 0 {
                lower_sum += x.get_lb() * a;
            } else {
                lower_sum += x.get_ub() * a;
            }
        }
        for (xx, a) in self.x.iter_mut().zip(self.a.iter().copied()) {
            if a == 0 {
                continue;
            }
            let mut x = xx.borrow_mut();
            if a > 0 {
                let up = self.b - lower_sum + x.get_lb() * a;
                x.set_ub(floor_div(up, a));
            } else {
                let down = -self.b + lower_sum - x.get_ub() * a;
                x.set_lb(ceil_div(down, -a));
            }
        }
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
