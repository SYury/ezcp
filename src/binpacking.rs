use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock, PropagatorState};
use crate::search::Search;
use crate::variable::Variable;
use std::cell::RefCell;
use std::cmp::max;
use std::rc::Rc;

pub struct BinPackingConstraint {
    assignment: Vec<Rc<RefCell<Variable>>>,
    load: Vec<Rc<RefCell<Variable>>>,
    weight: Vec<i64>,
}

impl BinPackingConstraint {
    pub fn new(
        assignment: Vec<Rc<RefCell<Variable>>>,
        load: Vec<Rc<RefCell<Variable>>>,
        weight: Vec<i64>,
    ) -> Self {
        Self {
            assignment,
            load,
            weight,
        }
    }
}

impl Constraint for BinPackingConstraint {
    fn satisfied(&self) -> bool {
        let mut load = vec![0; self.load.len()];
        for (i, var) in self.assignment.iter().enumerate() {
            if !var.borrow().is_assigned() {
                return false;
            }
            let bin = var.borrow().value();
            load[bin as usize] += self.weight[i];
        }
        for (i, var) in self.load.iter().enumerate() {
            if !var.borrow().is_assigned() {
                return false;
            }
            if load[i] != var.borrow().value() {
                return false;
            }
        }
        true
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(BinPackingPropagator::new(
            self.assignment.clone(),
            self.load.clone(),
            self.weight.clone(),
            index0,
        )))]
    }
}

pub struct BinPackingPropagator {
    pcb: PropagatorControlBlock,
    assignment: Vec<Rc<RefCell<Variable>>>,
    load: Vec<Rc<RefCell<Variable>>>,
    weight: Vec<i64>,
    total_weight: i64,
}

impl BinPackingPropagator {
    pub fn new(
        mut assignment: Vec<Rc<RefCell<Variable>>>,
        load: Vec<Rc<RefCell<Variable>>>,
        mut weight: Vec<i64>,
        id: usize,
    ) -> Self {
        let mut order = vec![(0i64, 0usize); weight.len()];
        let mut total_weight = 0;
        for i in 0..order.len() {
            total_weight += weight[i];
            order[i] = (weight[i], i);
        }
        order.sort();
        order.reverse();
        for i in 0..order.len() {
            weight[i] = order[i].0;
            if i == order[i].1 {
                continue;
            }
            let mut k = i;
            let mut j = order[i].1;
            let begin = assignment[i].clone();
            while j != i {
                assignment.swap(k, j);
                order[k].1 = k;
                k = j;
                j = order[k].1;
            }
            order[k].1 = k;
            assignment[k] = begin;
        }
        Self {
            pcb: PropagatorControlBlock::new(id),
            assignment,
            load,
            weight,
            total_weight,
        }
    }
}

fn no_sum(s: &[i64], l: i64, r: i64, l1: &mut i64, r1: &mut i64) -> bool {
    if l <= 0 || r >= s.iter().sum() {
        return false;
    }
    let n = s.len();
    let mut sa = 0;
    let mut sc = 0;
    let mut k = 0;
    let mut k1 = 0;

    while sc + s[n - k1 - 1] < l {
        sc += s[n - k1 - 1];
        k1 += 1;
    }
    let mut sb = s[n - k1 - 1];
    while sa < l && sb <= r {
        sa += s[k];
        k += 1;
        if sa < l {
            k1 -= 1;
            sc -= s[n - k1 - 1];
            sb += s[n - k1 - 1];
            while sa + sc >= l {
                k1 -= 1;
                sb += s[n - k1 - 1] - s[n - k1 - k - 2];
                sc -= s[n - k1 - 1];
            }
        }
    }
    *l1 = sa + sc;
    *r1 = sb;
    sa < l
}

fn bound(items: &[i64], capacity: i64) -> usize {
    let n = items.len();
    let mut big = 0;
    while big < n && 2 * items[big] > capacity {
        big += 1;
    }
    let mut big2 = big;
    while big2 < n && 3 * items[big2] > capacity {
        big2 += 1;
    }
    let mut h = 0;
    let mut ff = 0;
    for i in (big..big2).rev() {
        while ff < big && items[ff] + items[i] > capacity {
            ff += 1;
        }
        if ff < big {
            ff += 1;
        } else {
            h += 1;
        }
    }
    h = (h + 1) / 2;
    let mut mx = 0;
    let mut lptr = 0;
    let mut rptr: i32 = (n as i32) - 1;
    let mut lsum = 0;
    let mut rsum = 0;
    let sum: i64 = items.iter().sum();
    for v in 0..capacity / 3 + 1 {
        while lptr < big && items[lptr] > capacity - v {
            lsum += items[lptr];
            lptr += 1;
        }
        while rptr >= (big as i32) && items[rptr as usize] < v {
            rsum += items[rptr as usize];
            rptr -= 1;
        }
        let mut curr = sum - lsum - rsum - capacity * ((big - lptr + h) as i64);
        if curr >= 0 {
            curr = (curr + capacity - 1) / capacity;
        } else {
            curr = 0;
        }
        mx = max(mx, curr);
    }
    (mx as usize) + big + h
}

impl Propagator for BinPackingPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for v in &self.assignment {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
        for v in &self.load {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for v in &self.assignment {
            v.borrow_mut()
                .remove_listener(self_pointer.clone(), Event::Modified);
        }
        for v in &self.load {
            v.borrow_mut()
                .remove_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(
        &mut self,
        _self_pointer: Rc<RefCell<dyn Propagator>>,
        _search: &mut Search<'_>,
    ) -> PropagatorState {
        let items = self.assignment.len();
        let bins = self.load.len();
        let mut possible = vec![Vec::<usize>::new(); bins];
        let mut required = vec![Vec::<usize>::new(); bins];
        let mut candidate = vec![Vec::<usize>::new(); bins];
        let mut possible_sum = vec![0; bins];
        let mut required_sum = vec![0; bins];

        for i in 0..items {
            if self.assignment[i].borrow().is_assigned() {
                let bin = self.assignment[i].borrow().value() as usize;
                required[bin].push(i);
                required_sum[bin] += self.weight[i];
                possible[bin].push(i);
                possible_sum[bin] += self.weight[i];
            } else {
                for bin in self.assignment[i].borrow().iter() {
                    possible[bin as usize].push(i);
                    possible_sum[bin as usize] += self.weight[i];
                    candidate[bin as usize].push(i);
                }
            }
        }

        for j in 0..bins {
            let mut load = self.load[j].borrow_mut();
            load.set_lb(required_sum[j]);
            load.set_ub(possible_sum[j]);
        }

        let mut upper_sum = 0;
        let mut lower_sum = 0;
        for j in 0..bins {
            let load = self.load[j].borrow();
            upper_sum += load.get_ub();
            lower_sum += load.get_lb();
        }
        for j in 0..bins {
            let mut load = self.load[j].borrow_mut();
            let lb = load.get_lb();
            let ub = load.get_ub();
            load.set_lb(self.total_weight - upper_sum + ub);
            load.set_ub(self.total_weight - lower_sum + lb);
        }

        for j in 0..bins {
            let load = self.load[j].borrow();
            for i in candidate[j].iter().cloned() {
                let mut assign = self.assignment[i].borrow_mut();
                if required_sum[j] + self.weight[i] > load.get_ub() {
                    assign.remove(j as i64);
                } else if possible_sum[j] + self.weight[i] < load.get_lb() {
                    assign.assign(j as i64);
                }
            }
        }

        let mut l1 = 0;
        let mut r1 = 0;
        for j in 0..bins {
            let mut c = Vec::with_capacity(candidate[j].len());
            for cand in candidate[j].iter().cloned() {
                c.push(self.weight[cand]);
            }
            let mut load = self.load[j].borrow_mut();
            if no_sum(
                &c,
                load.get_lb() - required_sum[j],
                load.get_ub() - required_sum[j],
                &mut l1,
                &mut r1,
            ) {
                load.fail();
                return PropagatorState::Normal;
            }
            if no_sum(
                &c,
                load.get_lb() - required_sum[j],
                load.get_lb() - required_sum[j],
                &mut l1,
                &mut r1,
            ) {
                load.set_lb(required_sum[j] + r1);
            }
            if no_sum(
                &c,
                load.get_ub() - required_sum[j],
                load.get_ub() - required_sum[j],
                &mut l1,
                &mut r1,
            ) {
                load.set_ub(required_sum[j] + l1);
            }
        }

        for j in 0..bins {
            let load = self.load[j].borrow();
            for (pos, i) in candidate[j].iter().cloned().enumerate() {
                let mut assign = self.assignment[i].borrow_mut();
                let mut cand = candidate[j].clone();
                cand.remove(pos);
                let mut c = Vec::with_capacity(cand.len());
                for k in &cand {
                    c.push(self.weight[*k]);
                }
                if no_sum(
                    &c,
                    load.get_lb() - required_sum[j] - self.weight[i],
                    load.get_ub() - required_sum[j] - self.weight[i],
                    &mut l1,
                    &mut r1,
                ) {
                    assign.remove(j as i64);
                }
                if no_sum(
                    &c,
                    load.get_lb() - required_sum[j],
                    load.get_ub() - required_sum[j],
                    &mut l1,
                    &mut r1,
                ) {
                    assign.assign(j as i64);
                }
            }
        }

        let mut bin_capacity = 0;
        let mut unpacked = Vec::new();
        let mut fake = Vec::new();
        for load in &self.load {
            bin_capacity = max(bin_capacity, load.borrow().get_ub());
        }
        for i in 0..items {
            if !self.assignment[i].borrow().is_assigned() {
                unpacked.push(self.weight[i]);
            }
        }
        for (j, s) in required_sum.iter().enumerate() {
            let w = *s + bin_capacity - self.load[j].borrow().get_ub();
            if w > 0 {
                fake.push(w);
            }
        }
        fake.sort();
        fake.reverse();
        if unpacked.is_empty() && fake.is_empty() {
            return PropagatorState::Normal;
        }
        let mut all = Vec::with_capacity(unpacked.len() + fake.len());
        let mut i = 0;
        let mut j = 0;
        while i < unpacked.len() || j < fake.len() {
            if j == fake.len() || (i < unpacked.len() && fake[j] > unpacked[i]) {
                all.push(unpacked[i]);
                i += 1;
            } else {
                all.push(fake[j]);
                j += 1;
            }
        }
        if bound(&all, bin_capacity) > bins {
            self.assignment[0].borrow().fail();
        }
        PropagatorState::Normal
    }

    fn get_cb(&self) -> &PropagatorControlBlock {
        &self.pcb
    }

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock {
        &mut self.pcb
    }
}
