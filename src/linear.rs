use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock, PropagatorState};
use crate::search::Search;
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

    fn failed(&self) -> bool {
        let mut lb = 0;
        for (xx, a) in self.x.iter().zip(self.a.iter().copied()) {
            let x = xx.borrow();
            if a > 0 {
                lb += x.get_lb() * a;
            } else {
                lb += x.get_ub() * a;
            }
        }
        lb > self.b
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(LinearInequalityPropagator::new(
            self.x.clone(),
            self.a.clone(),
            self.b,
            index0,
        )))]
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

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for (i, x) in self.x.iter().enumerate() {
            if self.a[i] > 0 {
                x.borrow_mut()
                    .remove_listener(self_pointer.clone(), Event::LowerBound);
            } else {
                x.borrow_mut()
                    .remove_listener(self_pointer.clone(), Event::UpperBound);
            }
        }
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
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
        PropagatorState::Normal
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

// sum x[i] * a[i] != b
pub struct LinearNotEqualConstraint {
    x: Vec<Rc<RefCell<Variable>>>,
    a: Vec<i64>,
    b: i64,
}

impl LinearNotEqualConstraint {
    pub fn new(x: Vec<Rc<RefCell<Variable>>>, a: Vec<i64>, b: i64) -> Self {
        assert!(x.len() == a.len());
        Self { x, a, b }
    }
}

impl Constraint for LinearNotEqualConstraint {
    fn satisfied(&self) -> bool {
        let mut sum = 0;
        for (x, a) in self.x.iter().zip(self.a.iter()) {
            if !x.borrow().is_assigned() {
                return false;
            }
            sum += x.borrow().value() * (*a);
        }
        sum != self.b
    }

    fn failed(&self) -> bool {
        let mut lb = 0;
        let mut ub = 0;
        for (xx, a) in self.x.iter().zip(self.a.iter().copied()) {
            let x = xx.borrow();
            if a > 0 {
                lb += x.get_lb() * a;
                ub += x.get_ub() * a;
            } else {
                lb += x.get_ub() * a;
                ub += x.get_lb() * a;
            }
        }
        lb == ub && lb == self.b
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(LinearNotEqualPropagator::new(
            self.x.clone(),
            self.a.clone(),
            self.b,
            index0,
        )))]
    }
}

pub struct LinearNotEqualPropagator {
    pcb: PropagatorControlBlock,
    x: Vec<Rc<RefCell<Variable>>>,
    a: Vec<i64>,
    b: i64,
}

impl LinearNotEqualPropagator {
    pub fn new(x: Vec<Rc<RefCell<Variable>>>, a: Vec<i64>, b: i64, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            x,
            a,
            b,
        }
    }
}

impl Propagator for LinearNotEqualPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for (i, x) in self.x.iter().enumerate() {
            if self.a[i] != 0 {
                x.borrow_mut()
                    .add_listener(self_pointer.clone(), Event::Assigned);
            }
        }
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for (i, x) in self.x.iter().enumerate() {
            if self.a[i] != 0 {
                x.borrow_mut()
                    .remove_listener(self_pointer.clone(), Event::Assigned);
            }
        }
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
        let cnt = self.x.iter().filter(|v| v.borrow().is_assigned()).count();
        if cnt != self.x.len() - 1 {
            return PropagatorState::Normal;
        }
        let (pos, v) = self
            .x
            .iter()
            .enumerate()
            .find(|v| !v.1.borrow().is_assigned())
            .unwrap();
        if self.a[pos] == 0 {
            return PropagatorState::Normal;
        }
        let mut rem = self.b;
        for (a, x) in self.a.iter().copied().zip(self.x.iter()) {
            if x.borrow().is_assigned() {
                rem -= a * x.borrow().value();
            }
        }
        let val = rem / self.a[pos];
        if val * self.a[pos] == rem {
            v.borrow_mut().remove(val);
        }
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

// sum x[i] * a[i] == b
pub struct LinearEqualityConstraint {
    x: Vec<Rc<RefCell<Variable>>>,
    a: Vec<i64>,
    b: i64,
}

impl LinearEqualityConstraint {
    pub fn new(x: Vec<Rc<RefCell<Variable>>>, a: Vec<i64>, b: i64) -> Self {
        assert!(x.len() == a.len());
        Self { x, a, b }
    }
}

impl Constraint for LinearEqualityConstraint {
    fn satisfied(&self) -> bool {
        let mut sum = 0;
        for (x, a) in self.x.iter().zip(self.a.iter()) {
            if !x.borrow().is_assigned() {
                return false;
            }
            sum += x.borrow().value() * (*a);
        }
        sum == self.b
    }

    fn failed(&self) -> bool {
        let mut lb = 0;
        let mut ub = 0;
        for (xx, a) in self.x.iter().zip(self.a.iter().copied()) {
            let x = xx.borrow();
            if a > 0 {
                lb += x.get_lb() * a;
                ub += x.get_ub() * a;
            } else {
                lb += x.get_ub() * a;
                ub += x.get_lb() * a;
            }
        }
        ub < self.b || lb > self.b
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(LinearEqualityPropagator::new(
            self.x.clone(),
            self.a.clone(),
            self.b,
            index0,
        )))]
    }
}

pub struct LinearEqualityPropagator {
    pcb: PropagatorControlBlock,
    x: Vec<Rc<RefCell<Variable>>>,
    a: Vec<i64>,
    b: i64,
}

impl LinearEqualityPropagator {
    pub fn new(x: Vec<Rc<RefCell<Variable>>>, a: Vec<i64>, b: i64, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            x,
            a,
            b,
        }
    }
}

impl Propagator for LinearEqualityPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for (i, x) in self.x.iter().enumerate() {
            if self.a[i] != 0 {
                x.borrow_mut()
                    .add_listener(self_pointer.clone(), Event::LowerBound);
                x.borrow_mut()
                    .add_listener(self_pointer.clone(), Event::UpperBound);
            }
        }
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for (i, x) in self.x.iter().enumerate() {
            if self.a[i] != 0 {
                x.borrow_mut()
                    .remove_listener(self_pointer.clone(), Event::LowerBound);
                x.borrow_mut()
                    .remove_listener(self_pointer.clone(), Event::UpperBound);
            }
        }
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
        let mut lower_sum = 0;
        let mut upper_sum = 0;
        for (xx, a) in self.x.iter().zip(self.a.iter().copied()) {
            let x = xx.borrow();
            if a > 0 {
                lower_sum += x.get_lb() * a;
                upper_sum += x.get_ub() * a;
            } else {
                lower_sum += x.get_ub() * a;
                upper_sum += x.get_lb() * a;
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
                let down = self.b - upper_sum + x.get_ub() * a;
                x.set_lb(ceil_div(down, a));
            } else {
                let down = -self.b + lower_sum - x.get_ub() * a;
                x.set_lb(ceil_div(down, -a));
                let up = -self.b + upper_sum - x.get_lb() * a;
                x.set_ub(floor_div(up, -a));
            }
        }
        PropagatorState::Normal
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
