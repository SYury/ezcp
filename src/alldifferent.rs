use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock};
use crate::solver::Solver;
use std::cell::RefCell;
use std::collections::{BinaryHeap, HashSet};
use std::rc::Rc;
use crate::variable::Variable;

pub struct AllDifferentConstraint {
    vars: Vec<Rc<RefCell<Variable>>>
}

impl AllDifferentConstraint {
    pub fn new(vars: Vec<Rc<RefCell<Variable>>>) -> Self {
        Self {
            vars
        }
    }
}

impl Constraint for AllDifferentConstraint {
    fn satisfied(&self) -> bool {
        let mut vals = HashSet::new();
        for v in &self.vars {
            if v.borrow().is_assigned() {
                let val = v.borrow().value();
                if !vals.insert(val) {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
    fn create_propagators(&self, solver: &mut Solver) {
        let p = Rc::new(RefCell::new(AllDifferentACPropagator::new(self.vars.clone(), solver.new_propagator_id())));
        solver.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

struct SCC {
    gr: Vec<Vec<usize>>,
    grt: Vec<Vec<usize>>,
    used: Vec<bool>,
    order: Vec<usize>,
    rev_id_map: Vec<Vec<usize>>,
    comp: Vec<i32>,
    n: usize,
}

///implementation assumes that edge (v, u) is in matching iff v < u
impl SCC {
    pub fn new(gr: Vec<Vec<usize>>) -> Self {
        let n = gr.len();
        let mut rev_id_map = vec![Vec::<usize>::new(); n];
        let mut grt = vec![Vec::<usize>::new(); n];
        for i in 0..n {
            for (id, j) in gr[i].iter().cloned().enumerate() {
                grt[j].push(i);
                rev_id_map[j].push(id);
            }
        }
        Self {
            gr,
            grt,
            used: vec![false; n],
            order: Vec::new(),
            comp: vec![-1; n],
            rev_id_map,
            n,
        }
    }
    fn calc_order(&mut self, v: usize) {
        self.used[v] = true;
        for u in self.gr[v].clone().iter().cloned() {
            if !self.used[u] {
                self.calc_order(u);
            }
        }
        self.order.push(v);
    }
    fn mark_scc(&mut self, v: usize, scc: i32) {
        self.comp[v] = scc;
        for u in self.grt[v].clone().iter().cloned() {
            if self.comp[u] == -1 {
                self.mark_scc(u, scc);
            }
        }
    }
    fn find_scc(&mut self) {
        for i in 0..self.n {
            if !self.used[i] {
                self.calc_order(i);
            }
        }
        self.order.reverse();
        let mut curr = 0;
        for v in self.order.clone().iter().cloned() {
            if self.comp[v] == -1 {
                self.mark_scc(v, curr);
                curr += 1;
            }
        }
    }
    pub fn get_bad_edges(&mut self) -> Vec<(usize, usize)> {
        self.find_scc();
        let mut ok = vec![Vec::<bool>::new(); self.n];
        for v in 0..self.n {
            ok[v] = vec![false; self.gr[v].len()];
            for (i, u) in self.gr[v].iter().cloned().enumerate() {
                if self.comp[v] == self.comp[u] || v < u {
                    ok[v][i] = true;
                }
            }
        }
        for iter in 0..2 {
            let mut q = vec![0; self.n];
            let mut qh = 0;
            let mut qt = 0;
            let mut free = vec![true; self.n];
            for v in 0..self.n {
                for u in self.gr[v].iter().cloned() {
                    if v < u {
                        free[v] = false;
                        free[u] = false;
                    }
                }
            }
            for v in 0..self.n {
                if free[v] {
                    q[qt] = v;
                    qt += 1;
                }
            }
            while qh < qt {
                let v = q[qh];
                qh += 1;
                let it = match iter {
                    0 => self.gr[v].iter().cloned().enumerate(),
                    _ => self.grt[v].iter().cloned().enumerate(),
                };
                for (i, u) in it {
                    let edge = match iter {
                        0 => i,
                        _ => self.rev_id_map[v][i],
                    };
                    ok[v][edge] = true;
                    if !free[u] {
                        free[u] = true;
                        q[qt] = u;
                        qt += 1;
                    }
                }
            }
        }
        let mut ans = Vec::<(usize, usize)>::new();
        for v in 0..self.n {
            for (u, flag) in self.gr[v].iter().cloned().zip(ok[v].iter().cloned()) {
                if !flag {
                    ans.push((v, u));
                }
            }
        }
        ans
    }
}

struct FlowEdge {
    pub to: usize,
    pub flow: i32,
    pub capacity: i32,
}

impl FlowEdge {
    pub fn new(to: usize, capacity: i32) -> Self {
        Self {
            to,
            flow: 0,
            capacity,
        }
    }
}

struct ACMatching {
    s: usize,
    t: usize,
    edges: Vec<FlowEdge>,
    graph: Vec<Vec<usize>>,
    pub vals: Vec<i64>,
    ptr: Vec<usize>,
    level: Vec<i32>,
    q: Vec<usize>,
    qh: usize,
    qt: usize,
}

impl ACMatching {
    pub fn new(vars: &Vec<Rc<RefCell<Variable>>>) -> Self {
        let n = vars.len();
        let mut edges = Vec::<FlowEdge>::new();
        let mut graph = Vec::<Vec<usize>>::with_capacity(n);
        let mut vals = Vec::<i64>::new();
        let mut h = BinaryHeap::<(i64, usize)>::new();
        let mut it = Vec::<Box<dyn Iterator<Item = i64>>>::with_capacity(n);
        for i in 0..n {
            graph.push(Vec::new());
            it.push(Box::new(vars[i].borrow().iter()));
        }
        for (i, iter) in it.iter_mut().enumerate() {
            if let Some(val) = iter.next() {
                h.push((-val, i));
            }
        }
        while !h.is_empty() {
            let tmp = h.pop().unwrap();
            let mut i = tmp.1;
            let v = tmp.0;
            let vertex = vals.len() + n;
            vals.push(-v);
            graph.push(Vec::new());
            loop {
                let e = edges.len();
                edges.push(FlowEdge::new(vertex, 1));
                edges.push(FlowEdge::new(i, 0));
                graph[i].push(e);
                graph[vertex].push(e + 1);
                if let Some(nxt_val) = it[i].next() {
                    h.push((-nxt_val, i));
                }
                if h.is_empty() || h.peek().unwrap().0 != v {
                    break;
                }
                i = h.pop().unwrap().1;
            }
        }
        let s = graph.len();
        let t = s + 1;
        for _ in 0..2 {
            graph.push(Vec::new());
        }
        for i in 0..n {
            let e = edges.len();
            edges.push(FlowEdge::new(i, 1));
            edges.push(FlowEdge::new(s, 0));
            graph[s].push(e);
            graph[i].push(e + 1);
        }
        for i in n..vals.len()+n {
            let e = edges.len();
            edges.push(FlowEdge::new(t, 1));
            edges.push(FlowEdge::new(i, 0));
            graph[i].push(e);
            graph[t].push(e + 1);
        }
        let verts = graph.len();
        Self {
            s,
            t,
            edges,
            graph,
            vals,
            ptr: vec![0; verts],
            level: vec![-1; verts],
            q: vec![0; verts],
            qh: 0,
            qt: 0,
        }
    }
    pub fn bfs(&mut self) -> bool {
        while self.qh < self.qt {
            let v = self.q[self.qh];
            self.qh += 1;
            for id in self.graph[v].iter().cloned() {
                if self.edges[id].capacity == self.edges[id].flow {
                    continue;
                }
                if self.level[self.edges[id].to] != -1 {
                    continue;
                }
                self.level[self.edges[id].to] = self.level[v] + 1;
                self.q[self.qt] = self.edges[id].to;
                self.qt += 1;
            }
        }
        self.level[self.t] != -1
    }
    pub fn dfs(&mut self, v: usize, pushed: i32) -> i32 {
        if pushed == 0 {
            return 0;
        }
        if v == self.t {
            return pushed;
        }
        while self.ptr[v] < self.graph[v].len() {
            let id = self.graph[v][self.ptr[v]];
            let u = self.edges[id].to;
            if self.level[v] + 1 != self.level[u] || self.edges[id].capacity == self.edges[id].flow {
                self.ptr[v] += 1;
                continue;
            }
            let nxt = self.dfs(u, i32::min(pushed, self.edges[id].capacity - self.edges[id].flow));
            if nxt > 0 {
                self.edges[id].flow += nxt;
                self.edges[id^1].flow -= nxt;
                return nxt;
            }
            self.ptr[v] += 1;
        }
        0
    }
    pub fn matching(&mut self) -> Option<Vec<Vec<usize>>> {
        let mut flow = 0;
        loop {
            self.ptr.fill(0);
            self.level.fill(-1);
            self.level[self.s] = 0;
            self.q[0] = self.s;
            self.qh = 0;
            self.qt = 1;
            if !self.bfs() {
                break;
            }
            loop {
                let pushed = self.dfs(self.s, i32::MAX);
                if pushed > 0 {
                    flow += pushed;
                } else {
                    break;
                }
            }
        }
        if flow as usize != self.graph.len() - self.vals.len() - 2 {
            return None;
        }
        let mut ans = vec![Vec::<usize>::new(); self.graph.len() - 2];
        for v in 0..self.graph.len()-2-self.vals.len() {
            for id in self.graph[v].iter().cloned() {
                let u = self.edges[id].to;
                if self.edges[id].to < self.graph.len() - 2 && self.edges[id].capacity > 0 {
                    if self.edges[id].flow == self.edges[id].capacity {
                        ans[v].push(u);
                    } else {
                        ans[u].push(v);
                    }
                }
            }
        }
        Some(ans)
    }
}

pub struct AllDifferentACPropagator {
    pcb: PropagatorControlBlock,
    vars: Vec<Rc<RefCell<Variable>>>
}

impl AllDifferentACPropagator {
    pub fn new(vars: Vec<Rc<RefCell<Variable>>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock {
                has_new_events: false,
                queued: false,
                id
            },
            vars,
        }
    }
}

impl Propagator for AllDifferentACPropagator {

    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for v in &self.vars {
            v.borrow_mut().add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(&mut self) {
        let mut m = ACMatching::new(&self.vars);
        if let Some(g) = m.matching() {
            let mut scc = SCC::new(g);
            let mut edges = scc.get_bad_edges();
            for (val, i) in edges.drain(..) {
                self.vars[i].borrow_mut().remove(m.vals[val - self.vars.len()]);
            }
        } else {
            self.vars[0].borrow().fail();
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
