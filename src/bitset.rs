use crate::domain::{Domain, DomainState};
use crate::solver::SolverState;
use std::boxed::Box;
use std::cell::RefCell;
use std::rc::Rc;

pub struct BitsetDomain {
    solver_state: Rc<RefCell<SolverState>>,
    data: Vec<u64>,
    start: i64,
    first_block: usize,
    last_block: usize,
    size: u64,
    checkpoints: Vec<Vec<(usize, u64)>>,
    trail: Vec<(usize, u64)>,
    modified: Vec<usize>,
}

impl BitsetDomain {
    fn save(&mut self, block: usize) {
        if self.modified[block] >= self.trail.len() || self.trail[self.modified[block]].0 != block {
            self.modified[block] = self.trail.len();
            self.trail.push((block, self.data[block]));
        }
    }
}

pub struct BitsetDomainIterator<'a> {
    iter: std::slice::Iter<'a, u64>,
    remain: usize,
    block: u64,
    start: i64,
}

impl Iterator for BitsetDomainIterator<'_> {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        while self.block == 0 && self.remain > 0 {
            self.start += 64;
            self.remain -= 1;
            match self.iter.next() {
                None => {
                    return None;
                }
                Some(x) => {
                    self.block = *x;
                }
            }
        }
        if self.remain == 0 && self.block == 0 {
            return None;
        }
        let shift = self.block.trailing_zeros();
        self.block ^= 1u64 << shift;
        Some(self.start + (shift as i64))
    }
}

impl Domain for BitsetDomain {
    fn new(solver_state: Rc<RefCell<SolverState>>, lb: i64, ub: i64) -> Self {
        let size = (ub - lb + 1) as u64;
        let blocks = (size / 64 + ((size % 64 > 0) as u64)) as usize;
        let mut data = vec![u64::MAX; blocks];
        if size % 64 > 0 {
            let x = size % 64;
            data[blocks - 1] = (1u64 << x) - 1;
        }
        Self {
            solver_state,
            data,
            start: lb,
            first_block: 0,
            last_block: blocks - 1,
            size,
            checkpoints: Vec::new(),
            trail: Vec::with_capacity(blocks),
            modified: vec![0; blocks],
        }
    }

    fn assign(&mut self, x: i64) -> DomainState {
        if x < self.start || x >= self.start + (self.data.len() as i64) * 64 {
            self.solver_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        let id = (x - self.start) as u64;
        let block = (id / 64) as usize;
        let shift = id - (block as u64) * 64;
        if 0 == (self.data[block] & (1u64 << shift)) {
            self.solver_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        if self.size == 1 {
            return DomainState::Same;
        }
        for i in self.first_block..self.last_block + 1 {
            if i == block || self.data[i] != 0 {
                self.save(i);
            }
            self.data[i] = 0;
        }
        self.size = 1;
        self.data[block] = 1u64 << shift;
        self.first_block = block;
        self.last_block = block;
        DomainState::Modified
    }

    fn is_assigned(&self) -> bool {
        self.size == 1
    }

    fn possible(&self, x: i64) -> bool {
        if x < self.start || x >= self.start + (self.data.len() as i64) * 64 {
            return false;
        }
        let id = (x - self.start) as u64;
        let block = (id / 64) as usize;
        let shift = id - (block as u64) * 64;
        0 != (self.data[block] & (1u64 << shift))
    }

    fn remove(&mut self, x: i64) -> DomainState {
        if x < self.start || x >= self.start + (self.data.len() as i64) * 64 {
            return DomainState::Same;
        }
        let id = (x - self.start) as u64;
        let block = (id / 64) as usize;
        let shift = id - (block as u64) * 64;
        if 0 == (self.data[block] & (1u64 << shift)) {
            return DomainState::Same;
        }
        self.save(block);
        self.data[block] ^= 1u64 << shift;
        self.size -= 1;
        if self.size == 0 {
            self.solver_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        while self.data[self.first_block] == 0 {
            self.first_block += 1;
        }
        while self.data[self.last_block] == 0 {
            self.last_block -= 1;
        }
        DomainState::Modified
    }

    fn get_lb(&self) -> i64 {
        let shift = self.data[self.first_block].trailing_zeros();
        self.start + ((self.first_block as i64) * 64 + (shift as i64))
    }

    fn get_ub(&self) -> i64 {
        let shift = 63 - self.data[self.last_block].leading_zeros();
        self.start + ((self.last_block as i64) * 64 + (shift as i64))
    }

    fn set_lb(&mut self, x: i64) -> DomainState {
        if x <= self.get_lb() {
            return DomainState::Same;
        }
        if x > self.get_ub() {
            self.solver_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        let id = (x - self.start) as u64;
        let block = (id / 64) as usize;
        let shift = id - (block as u64) * 64;
        if block >= self.first_block {
            let old_size = self.size;
            for i in self.first_block..block {
                if self.data[i] != 0 {
                    self.save(i);
                    self.size -= self.data[i].count_ones() as u64;
                    self.data[i] = 0;
                }
            }
            if shift > (self.data[block].trailing_zeros() as u64) {
                self.save(block);
                self.size -= (self.data[block] & ((1u64 << shift) - 1)).count_ones() as u64;
                self.data[block] &= !((1u64 << shift) - 1);
            }
            if self.size == 0 {
                self.solver_state.borrow_mut().fail();
                return DomainState::Failed;
            }
            while self.data[self.first_block] == 0 {
                self.first_block += 1;
            }
            if old_size != self.size {
                return DomainState::Modified;
            }
        }
        DomainState::Same
    }

    fn set_ub(&mut self, x: i64) -> DomainState {
        if x < self.get_lb() {
            self.solver_state.borrow_mut().fail();
            return DomainState::Failed;
        }
        if x >= self.get_ub() {
            return DomainState::Same;
        }
        let id = (x - self.start) as u64;
        let block = (id / 64) as usize;
        let shift = id - (block as u64) * 64;
        if block <= self.last_block {
            let old_size = self.size;
            for i in block + 1..self.last_block + 1 {
                if self.data[i] != 0 {
                    self.save(i);
                    self.size -= self.data[i].count_ones() as u64;
                    self.data[i] = 0;
                }
            }
            if (self.data[block].leading_zeros() as u64) < 63 - shift {
                self.save(block);
                self.size -= (self.data[block] & !((2u64 << shift) - 1)).count_ones() as u64;
                self.data[block] &= (2u64 << shift) - 1;
            }
            if self.size == 0 {
                self.solver_state.borrow_mut().fail();
                return DomainState::Failed;
            }
            while self.data[self.last_block] == 0 {
                self.last_block -= 1;
            }
            if old_size != self.size {
                return DomainState::Modified;
            }
        }
        DomainState::Same
    }

    fn checkpoint(&mut self) {
        self.checkpoints.push(self.trail.drain(..).collect());
    }

    fn rollback(&mut self) {
        for (i, old) in self.trail.drain(..) {
            let delta = old ^ self.data[i];
            if delta == 0 {
                continue;
            }
            self.size += delta.count_ones() as u64;
            self.data[i] ^= delta;
            if i < self.first_block {
                self.first_block = i;
            }
            if i > self.last_block {
                self.last_block = i;
            }
        }
        self.trail = self.checkpoints.pop().unwrap();
    }

    fn iter(&self) -> Box<dyn Iterator<Item = i64> + '_> {
        Box::new(BitsetDomainIterator {
            iter: self.data.as_slice()[self.first_block..].iter(),
            block: 0,
            remain: self.last_block - self.first_block + 1,
            start: self.start + (64 * self.first_block as i64) - 64,
        })
    }

    fn size(&self) -> u64 {
        self.size
    }
}
