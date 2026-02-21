use crate::search::SearchState;
use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(PartialEq, Eq)]
pub enum DomainState {
    Same,
    Modified,
    Failed,
}

pub trait Domain {
    fn new(search_state: Rc<RefCell<SearchState>>, lb: i64, ub: i64) -> Self
    where
        Self: Sized;
    fn assign(&mut self, x: i64) -> DomainState;
    fn is_assigned(&self) -> bool;
    fn remove(&mut self, x: i64) -> DomainState;
    fn possible(&self, x: i64) -> bool;
    fn get_lb(&self) -> i64;
    fn get_ub(&self) -> i64;
    fn get_median(&self) -> i64;
    fn set_lb(&mut self, x: i64) -> DomainState;
    fn set_ub(&mut self, x: i64) -> DomainState;
    fn checkpoint(&mut self);
    fn rollback(&mut self);
    fn rollback_all(&mut self);
    fn iter(&self) -> Box<dyn Iterator<Item = i64> + '_>;
    fn size(&self) -> u64;
}

/// implementation for domains which fit in {0, ..., 63}
pub struct SmallDomain {
    search_state: Rc<RefCell<SearchState>>,
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

impl SmallDomain {
    fn discard(&mut self, x: u8) {
        self.body &= !(1_u64 << x);
    }
}

impl Domain for SmallDomain {
    fn new(search_state: Rc<RefCell<SearchState>>, lb: i64, ub: i64) -> Self {
        let body = match ub - lb {
            63 => !0_u64,
            _ => (1_u64 << (ub - lb + 1)) - 1,
        };
        Self {
            search_state,
            body,
            start: lb,
            lb: 0,
            ub: (ub - lb) as u8,
            checkpoints: Vec::new(),
        }
    }
    fn assign(&mut self, x: i64) -> DomainState {
        if x < self.start || x >= self.start + 64 {
            self.search_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        let v = (x - self.start) as u8;
        if (self.body & (1_u64 << v)) == 0 {
            self.search_state.borrow_mut().fail();
            DomainState::Failed
        } else {
            let modified = self.body != 1_u64 << v;
            self.body = 1_u64 << v;
            self.lb = v;
            self.ub = v;
            if modified {
                DomainState::Modified
            } else {
                DomainState::Same
            }
        }
    }
    fn is_assigned(&self) -> bool {
        self.body.count_ones() == 1
    }

    fn possible(&self, x: i64) -> bool {
        if x < self.start || x >= self.start + 64 {
            return false;
        }
        let v = (x - self.start) as u8;
        self.body & (1u64 << v) > 0
    }

    fn remove(&mut self, x: i64) -> DomainState {
        if x < self.start || x >= self.start + 64 {
            return DomainState::Same;
        }
        let v = (x - self.start) as u8;
        let modified = self.body & (1u64 << v) > 0;
        self.discard(v);
        if self.body == 0 {
            self.search_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        if v == self.lb && self.body > 0 {
            self.lb = self.body.trailing_zeros() as u8;
        }
        if v == self.ub && self.body > 0 {
            self.ub = 63 - self.body.leading_zeros() as u8;
        }
        if modified {
            DomainState::Modified
        } else {
            DomainState::Same
        }
    }
    fn get_lb(&self) -> i64 {
        (self.lb as i64) + self.start
    }
    fn get_ub(&self) -> i64 {
        (self.ub as i64) + self.start
    }
    fn get_median(&self) -> i64 {
        let id = (self.size() - 1) / 2;
        let mut body = self.body;
        for _ in 0..id {
            let j = body.trailing_zeros() as u8;
            body ^= 1u64 << j;
        }
        (body.trailing_zeros() as i64) + self.start
    }
    fn set_lb(&mut self, x: i64) -> DomainState {
        if x <= self.get_lb() {
            return DomainState::Same;
        }
        if x > self.get_ub() {
            self.search_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        let y = x - self.start;
        let y1 = y as u8;
        for i in self.lb..y1 {
            self.discard(i);
        }
        self.lb = self.body.trailing_zeros() as u8;
        DomainState::Modified
    }
    fn set_ub(&mut self, x: i64) -> DomainState {
        if x < self.get_lb() {
            self.search_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        if x >= self.get_ub() {
            return DomainState::Same;
        }
        let y = x - self.start;
        let y1 = y as u8;
        for i in y1 + 1..self.ub + 1 {
            self.discard(i);
        }
        self.ub = 63 - self.body.leading_zeros() as u8;
        DomainState::Modified
    }
    fn checkpoint(&mut self) {
        self.checkpoints
            .push((self.body, self.start, self.lb, self.ub));
    }
    fn rollback(&mut self) {
        let state = self.checkpoints.pop().unwrap();
        self.body = state.0;
        self.start = state.1;
        self.lb = state.2;
        self.ub = state.3;
    }
    fn rollback_all(&mut self) {
        while !self.checkpoints.is_empty() {
            self.rollback();
        }
    }
    fn iter(&self) -> Box<dyn Iterator<Item = i64> + '_> {
        Box::new(SmallDomainIterator {
            body: self.body,
            start: self.start,
        })
    }
    fn size(&self) -> u64 {
        self.body.count_ones() as u64
    }
}
