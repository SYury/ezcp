use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock, PropagatorState};
use crate::scc::compute_scc;
use crate::search::Search;
use crate::variable::Variable;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

// assumes that flow graph is rooted at vertex 0
pub struct DominatorTree {
    gr: Vec<Vec<usize>>,
    grt: Vec<Vec<usize>>,
    idom: Vec<usize>,
    sdom: Vec<usize>,
    child: Vec<usize>,
    anc: Vec<usize>,
    order: Vec<usize>,
    label: Vec<usize>,
    p: Vec<usize>,
    bucket: Vec<Vec<usize>>,
    ptr: usize,
}

impl DominatorTree {
    pub fn new(gr: Vec<Vec<usize>>) -> Self {
        Self {
            gr,
            grt: Vec::new(),
            idom: Vec::new(),
            sdom: Vec::new(),
            child: Vec::new(),
            anc: Vec::new(),
            order: Vec::new(),
            label: Vec::new(),
            p: Vec::new(),
            bucket: Vec::new(),
            ptr: 0,
        }
    }

    fn dfs(&mut self, v: usize) {
        self.sdom[v] = self.ptr;
        self.order[self.ptr] = v;
        self.ptr += 1;
        for u in self.gr[v].clone().drain(..) {
            if self.sdom[u] == usize::MAX {
                self.p[u] = v;
                self.dfs(u);
            }
            self.grt[u].push(v);
        }
    }

    fn compress(&mut self, v: usize) {
        if self.anc[self.anc[v]] != usize::MAX {
            self.compress(self.anc[v]);
            if self.sdom[self.label[self.anc[v]]] < self.sdom[self.label[v]] {
                self.label[v] = self.label[self.anc[v]];
            }
            self.anc[v] = self.anc[self.anc[v]];
        }
    }

    fn eval(&mut self, v: usize) -> usize {
        if self.anc[v] == usize::MAX {
            v
        } else {
            self.compress(v);
            self.label[v]
        }
    }

    pub fn build(&mut self) {
        let mut n = self.gr.len();
        self.ptr = 0;
        self.grt = vec![Vec::new(); n];
        self.idom = vec![usize::MAX; n];
        self.sdom = vec![usize::MAX; n];
        self.child = vec![usize::MAX; n];
        self.anc = vec![usize::MAX; n];
        self.order = vec![usize::MAX; n];
        self.p = vec![usize::MAX; n];
        self.label = vec![usize::MAX; n];
        self.bucket = vec![Vec::new(); n];
        for i in 0..n {
            self.label[i] = i;
        }
        self.dfs(0);
        n = self.ptr;
        for j in (1..n).rev() {
            let v = self.order[j];
            for u in self.grt[v].clone().drain(..) {
                let w = self.eval(u);
                if self.sdom[w] < self.sdom[v] {
                    self.sdom[v] = self.sdom[w];
                }
            }
            self.bucket[self.order[self.sdom[v]]].push(v);
            self.anc[v] = self.p[v];
            let tmp: Vec<_> = self.bucket[self.p[v]].drain(..).collect();
            for u in tmp {
                let w = self.eval(u);
                if self.sdom[w] < self.sdom[u] {
                    self.idom[u] = w;
                } else {
                    self.idom[u] = self.p[v];
                }
            }
        }
        for i in 1..n {
            let v = self.order[i];
            if self.idom[v] != self.order[self.sdom[v]] {
                self.idom[v] = self.idom[self.idom[v]];
            }
        }
        self.idom[0] = 0;
    }

    pub fn get_dominators(&self) -> Vec<usize> {
        self.idom.clone()
    }
}

fn traverse_tree(
    tree: &Vec<Vec<usize>>,
    tin: &mut Vec<usize>,
    tout: &mut Vec<usize>,
    time: &mut usize,
    v: usize,
    p: usize,
) {
    tin[v] = *time;
    *time += 1;
    for u in tree[v].iter().cloned() {
        if u != p {
            traverse_tree(tree, tin, tout, time, u, v);
        }
    }
    tout[v] = *time;
    *time += 1;
}

pub struct TreeConstraint {
    ntree: Rc<RefCell<Variable>>,
    parent: Vec<Rc<RefCell<Variable>>>,
}

impl TreeConstraint {
    pub fn new(ntree: Rc<RefCell<Variable>>, parent: Vec<Rc<RefCell<Variable>>>) -> Self {
        Self { ntree, parent }
    }
}

impl Constraint for TreeConstraint {
    fn satisfied(&self) -> bool {
        if !self.ntree.borrow().is_assigned() {
            return false;
        }
        let ntree = self.ntree.borrow().value() as usize;
        let mut out = vec![Vec::new(); self.parent.len()];
        let mut trees = Vec::new();
        let mut used = vec![false; self.parent.len()];
        for i in 0..self.parent.len() {
            if !self.parent[i].borrow().is_assigned() {
                return false;
            }
            let j = self.parent[i].borrow().value() as usize;
            if i != j {
                out[j].push(i);
            } else {
                trees.push(i);
            }
        }
        for v in trees.iter().cloned() {
            if used[v] {
                return false;
            }
            let mut q = VecDeque::new();
            q.push_back(v);
            used[v] = true;
            while !q.is_empty() {
                let u = *q.front().unwrap();
                q.pop_front();
                for w in out[u].drain(..) {
                    if used[w] {
                        return false;
                    }
                    used[w] = true;
                    q.push_back(w);
                }
            }
        }
        ntree == trees.len()
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(TreePropagator::new(
            self.ntree.clone(),
            self.parent.clone(),
            index0,
        )))]
    }
}

pub struct TreePropagator {
    pcb: PropagatorControlBlock,
    ntree: Rc<RefCell<Variable>>,
    parent: Vec<Rc<RefCell<Variable>>>,
}

impl TreePropagator {
    pub fn new(
        ntree: Rc<RefCell<Variable>>,
        parent: Vec<Rc<RefCell<Variable>>>,
        id: usize,
    ) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            ntree,
            parent,
        }
    }
}

impl Propagator for TreePropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.ntree
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        for v in &self.parent {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.ntree
            .borrow_mut()
            .remove_listener(self_pointer.clone(), Event::Modified);
        for v in &self.parent {
            v.borrow_mut()
                .remove_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
        let n = self.parent.len();
        if n == 1 {
            return PropagatorState::Terminated;
        }
        let mut ext_gr = vec![Vec::new(); n + 1];
        let mut gr = vec![Vec::new(); n];
        let mut mintree = 0;
        let mut maxtree = 0;
        for (v, var) in self.parent.iter().enumerate() {
            for u in var.borrow().iter().map(|x| x as usize) {
                if u != v {
                    gr[v].push(u);
                    ext_gr[u + 1].push(v + 1);
                } else {
                    maxtree += 1;
                    ext_gr[0].push(v + 1);
                    ext_gr[v + 1].push(0);
                }
            }
        }
        let comps = compute_scc(&gr);
        let mut comp_id = vec![0; n];
        let mut sink = vec![true; comps.len()];
        for (i, comp) in comps.iter().enumerate() {
            for v in comp.iter().cloned() {
                comp_id[v] = i;
            }
        }
        for v in 0..n {
            for u in gr[v].iter().cloned() {
                if comp_id[v] != comp_id[u] {
                    sink[comp_id[v]] = false;
                    break;
                }
            }
        }
        for x in &sink {
            if *x {
                mintree += 1;
            }
        }
        if !self.ntree.borrow_mut().set_lb(mintree) {
            return PropagatorState::Normal;
        }
        if !self.ntree.borrow_mut().set_ub(maxtree) {
            return PropagatorState::Normal;
        }
        let mut dt = DominatorTree::new(ext_gr);
        dt.build();
        let dom = dt.get_dominators();
        if dom.contains(&usize::MAX) {
            self.parent[0].borrow_mut().fail();
            return PropagatorState::Normal;
        }
        let mut tree = vec![Vec::new(); n + 1];
        let mut tin = vec![0; n + 1];
        let mut tout = vec![0; n + 1];
        for (i, d) in dom.iter().cloned().enumerate() {
            if i != d {
                tree[d].push(i);
            }
        }
        let mut time = 0;
        traverse_tree(&tree, &mut tin, &mut tout, &mut time, 0, usize::MAX);
        for v in 0..n {
            for u in gr[v].iter().cloned() {
                if tin[v + 1] < tin[u + 1] && tout[v + 1] > tout[u + 1] {
                    self.parent[v].borrow_mut().remove(u as i64);
                }
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
