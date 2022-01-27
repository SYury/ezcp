use crate::solver::SolverState;
use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

pub trait Domain: {
    fn new(solver_state: Rc<RefCell<SolverState>>, lb: i64, ub: i64) -> Self where Self: Sized;
    fn assign(&mut self, x: i64);
    fn is_assigned(&self) -> bool;
    fn discard(&mut self, x: u8);
    fn remove(&mut self, x: i64);
    fn get_lb(&self) -> i64;
    fn get_ub(&self) -> i64;
    fn set_lb(&mut self, x: i64);
    fn set_ub(&mut self, x: i64);
    fn checkpoint(&mut self);
    fn rollback(&mut self);
    fn iter(&self) -> Box<dyn Iterator<Item = i64>>;
    fn size(&self) -> u64;
}

/// implementation for domains which fit in {0, ..., 63}
pub struct SmallDomain {
    solver_state: Rc<RefCell<SolverState>>,
    body: u64,
    start: i64,
    lb: u8,
    ub: u8,
    checkpoints: Vec<(u64, i64, u8, u8)>,
}

pub struct SmallDomainIterator {
    body: u64,
    start: i64,
}

impl Iterator for SmallDomainIterator {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.body == 0 {
            None
        } else {
            let add = self.body.trailing_zeros();
            self.body ^= 1u64 << add;
            Some(self.start + (add as i64))
        }
    }
}

impl Domain for SmallDomain {
    fn new(solver_state: Rc<RefCell<SolverState>>, lb: i64, ub: i64) -> Self {
        let body = match ub - lb {
            63 => !(0 as u64),
            _ => ((1 as u64) << (ub - lb + 1)) - 1,
        };
        Self {
            solver_state,
            body,
            start: lb,
            lb: 0,
            ub: (ub - lb) as u8,
            checkpoints: Vec::new(),
        }
    }
    fn assign(&mut self, x: i64) {
        let v = (x - self.start) as u8;
        if (self.body & ((1 as u64) << v)) == 0 {
            self.solver_state.borrow_mut().fail();
        } else {
            self.body = (1 as u64) << v;
            self.lb = v;
            self.ub = v;
        }
    }
    fn is_assigned(&self) -> bool {
        self.body.count_ones() == 1
    }
    fn discard(&mut self, x: u8) {
        self.body &= !((1 as u64) << x);
    }
    fn remove(&mut self, x: i64) {
        let v = (x - self.start) as u8;
        self.discard(v);
        if v == self.lb && self.body > 0 {
            self.lb = self.body.trailing_zeros() as u8;
        }
        if v == self.ub && self.body > 0 {
            self.ub = 63 - self.body.leading_zeros() as u8;
        }
    }
    fn get_lb(&self) -> i64 {
        (self.lb as i64) + self.start
    }
    fn get_ub(&self) -> i64 {
        (self.ub as i64) + self.start
    }
    fn set_lb(&mut self, x: i64) {
        let y = x - self.start;
        if y < 0 || y > 63 {
            self.solver_state.borrow_mut().fail();
        } else {
            let y1 = y as u8;
            if y1 > self.lb {
                for i in self.lb..y1 {
                    self.discard(i);
                }
                self.lb = y1;
            }
        }
    }
    fn set_ub(&mut self, x: i64) {
        let y = x - self.start;
        if y < 0 || y > 63 {
            self.solver_state.borrow_mut().fail();
        } else {
            let y1 = y as u8;
            if y1 < self.ub {
                for i in y1+1..self.ub+1 {
                    self.discard(i);
                }
                self.ub = y1;
            }
        }
    }
    fn checkpoint(&mut self) {
        self.checkpoints.push((self.body, self.start, self.lb, self.ub));
    }
    fn rollback(&mut self) {
        let state = self.checkpoints.pop().unwrap();
        self.body = state.0;
        self.start = state.1;
        self.lb = state.2;
        self.ub = state.3;
    }
    fn iter(&self) -> Box<dyn Iterator<Item = i64>> {
        Box::new(SmallDomainIterator {
            body: self.body.clone(),
            start: self.start.clone(),
        })
    }
    fn size(&self) -> u64 {
        self.body.count_ones() as u64
    }
}
