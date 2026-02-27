use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock, PropagatorState};
use crate::search::Search;
use crate::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

/// x +- y = C
pub struct SimpleArithmeticConstraint {
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
    c: i64,
    plus: bool,
}

impl SimpleArithmeticConstraint {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>, c: i64, plus: bool) -> Self {
        Self { x, y, c, plus }
    }
}

impl Constraint for SimpleArithmeticConstraint {
    fn satisfied(&self) -> bool {
        if !self.x.borrow().is_assigned() || !self.y.borrow().is_assigned() {
            false
        } else if self.plus {
            self.x.borrow().value() + self.y.borrow().value() == self.c
        } else {
            self.x.borrow().value() - self.y.borrow().value() == self.c
        }
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(SimpleArithmeticPropagator::new(
            self.x.clone(),
            self.y.clone(),
            self.c,
            self.plus,
            index0,
        )))]
    }
}

pub struct SimpleArithmeticPropagator {
    pcb: PropagatorControlBlock,
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
    c: i64,
    plus: bool,
}

impl SimpleArithmeticPropagator {
    pub fn new(
        x: Rc<RefCell<Variable>>,
        y: Rc<RefCell<Variable>>,
        c: i64,
        plus: bool,
        id: usize,
    ) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            x,
            y,
            c,
            plus,
        }
    }
}

impl Propagator for SimpleArithmeticPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.x
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        self.y
            .borrow_mut()
            .add_listener(self_pointer, Event::Modified);
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.x
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::Modified);
        self.y
            .borrow_mut()
            .remove_listener(self_pointer, Event::Modified);
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
        let mut x_vec = Vec::with_capacity(self.x.borrow().size() as usize);
        let mut y_vec = Vec::with_capacity(self.y.borrow().size() as usize);
        for val in self.x.borrow().iter() {
            x_vec.push(val);
        }
        for val in self.y.borrow().iter() {
            y_vec.push(val);
        }
        if self.plus {
            y_vec.reverse();
            let mut it_x = x_vec.iter().cloned();
            let mut it_y = y_vec.iter().cloned();
            let mut x = match it_x.next() {
                Some(x) => x,
                None => {
                    self.x.borrow().fail();
                    return PropagatorState::Normal;
                }
            };
            let mut y = match it_y.next() {
                Some(y) => y,
                None => {
                    self.y.borrow().fail();
                    return PropagatorState::Normal;
                }
            };
            loop {
                if x < self.c - y {
                    self.x.borrow_mut().remove(x);
                    if let Some(new_x) = it_x.next() {
                        x = new_x;
                    } else {
                        self.y.borrow_mut().remove(y);
                        break;
                    }
                } else if y > self.c - x {
                    self.y.borrow_mut().remove(y);
                    if let Some(new_y) = it_y.next() {
                        y = new_y;
                    } else {
                        self.x.borrow_mut().remove(x);
                        break;
                    }
                } else {
                    if let Some(new_x) = it_x.next() {
                        x = new_x;
                    } else {
                        break;
                    }
                    if let Some(new_y) = it_y.next() {
                        y = new_y;
                    } else {
                        self.x.borrow_mut().remove(x);
                        break;
                    }
                }
            }
            for rem_x in it_x {
                self.x.borrow_mut().remove(rem_x);
            }
            for rem_y in it_y {
                self.y.borrow_mut().remove(rem_y);
            }
        } else {
            let mut it_x = x_vec.iter().cloned();
            let mut it_y = y_vec.iter().cloned();
            let mut x = match it_x.next() {
                Some(x) => x,
                None => {
                    self.x.borrow().fail();
                    return PropagatorState::Normal;
                }
            };
            let mut y = match it_y.next() {
                Some(y) => y,
                None => {
                    self.y.borrow().fail();
                    return PropagatorState::Normal;
                }
            };
            loop {
                if x < y + self.c {
                    self.x.borrow_mut().remove(x);
                    if let Some(new_x) = it_x.next() {
                        x = new_x;
                    } else {
                        self.y.borrow_mut().remove(y);
                        break;
                    }
                } else if y < x - self.c {
                    self.y.borrow_mut().remove(y);
                    if let Some(new_y) = it_y.next() {
                        y = new_y;
                    } else {
                        self.x.borrow_mut().remove(x);
                        break;
                    }
                } else {
                    if let Some(new_x) = it_x.next() {
                        x = new_x;
                    } else {
                        break;
                    }
                    if let Some(new_y) = it_y.next() {
                        y = new_y;
                    } else {
                        self.x.borrow_mut().remove(x);
                        break;
                    }
                }
            }
            for rem_x in it_x {
                self.x.borrow_mut().remove(rem_x);
            }
            for rem_y in it_y {
                self.y.borrow_mut().remove(rem_y);
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
        true
    }
}

/// x = abs(y)
pub struct AbsConstraint {
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
}

impl AbsConstraint {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>) -> Self {
        Self { x, y }
    }
}

impl Constraint for AbsConstraint {
    fn satisfied(&self) -> bool {
        if !self.x.borrow().is_assigned() || !self.y.borrow().is_assigned() {
            return false;
        }
        self.x.borrow().value() == self.y.borrow().value().abs()
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        if self.x.as_ptr() == self.y.as_ptr() {
            return vec![];
        }
        vec![Rc::new(RefCell::new(AbsPropagator::new(
            self.x.clone(),
            self.y.clone(),
            index0,
        )))]
    }
}

pub struct AbsPropagator {
    pcb: PropagatorControlBlock,
    x: Rc<RefCell<Variable>>,
    y: Rc<RefCell<Variable>>,
}

impl AbsPropagator {
    pub fn new(x: Rc<RefCell<Variable>>, y: Rc<RefCell<Variable>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            x,
            y,
        }
    }
}

impl Propagator for AbsPropagator {
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
            .add_listener(self_pointer, Event::UpperBound);
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
            .remove_listener(self_pointer, Event::UpperBound);
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
        let mut x = self.x.borrow_mut();
        let mut y = self.y.borrow_mut();
        if x.get_lb() < 0 {
            x.set_lb(0);
        }
        x.set_lb(y.get_lb().max(-y.get_ub()));
        x.set_ub(y.get_ub().max(-y.get_lb()));
        y.set_lb(-x.get_ub());
        y.set_ub(x.get_ub());
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
