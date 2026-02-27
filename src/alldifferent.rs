use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock, PropagatorState};
use crate::search::Search;
use crate::variable::Variable;
use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::rc::Rc;

pub struct AllDifferentConstraint {
    vars: Vec<Rc<RefCell<Variable>>>,
}

impl AllDifferentConstraint {
    pub fn new(vars: Vec<Rc<RefCell<Variable>>>) -> Self {
        Self { vars }
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

    fn failed(&self) -> bool {
        let mut vals = HashSet::new();
        for v in &self.vars {
            if let Some(x) = v.borrow().try_value() {
                if !vals.insert(x) {
                    return true;
                }
            }
        }
        false
    }

    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>> {
        vec![Rc::new(RefCell::new(AllDifferentACPropagator::new(
            self.vars.clone(),
            index0,
        )))]
    }
}

pub struct SCC {
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
        for (i, row) in gr.iter().enumerate() {
            for (id, j) in row.iter().cloned().enumerate() {
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
        for (v, row) in self.gr.iter().enumerate() {
            ok[v] = vec![false; row.len()];
            for (i, u) in row.iter().cloned().enumerate() {
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
            for (v, f) in free.iter().enumerate() {
                if *f {
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
        for (v, row) in self.gr.iter().enumerate() {
            for (u, flag) in row.iter().cloned().zip(ok[v].iter().cloned()) {
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

pub(crate) struct ACMatching {
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

pub(crate) enum MatchingReturnValue {
    MatchingGraph,
    FlowGraph,
}

impl ACMatching {
    pub fn new(vars: &[Rc<RefCell<Variable>>], count: Option<&HashMap<i64, i32>>) -> Self {
        let n = vars.len();
        let mut edges = Vec::<FlowEdge>::new();
        let mut graph = Vec::<Vec<usize>>::with_capacity(n);
        let mut vals = Vec::<i64>::new();
        let mut h = BinaryHeap::<(i64, usize)>::new();
        let mut borrowed_vars = Vec::with_capacity(n);
        let mut it = Vec::<Box<dyn Iterator<Item = i64> + '_>>::with_capacity(n);
        for v in vars {
            borrowed_vars.push(v.borrow());
        }
        for var in &borrowed_vars {
            graph.push(Vec::new());
            it.push(var.iter());
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
        for i in n..vals.len() + n {
            let e = edges.len();
            if let Some(map) = count {
                edges.push(FlowEdge::new(t, *map.get(&vals[i - n]).unwrap()));
            } else {
                edges.push(FlowEdge::new(t, 1));
            }
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
            if self.level[v] + 1 != self.level[u] || self.edges[id].capacity == self.edges[id].flow
            {
                self.ptr[v] += 1;
                continue;
            }
            let nxt = self.dfs(
                u,
                i32::min(pushed, self.edges[id].capacity - self.edges[id].flow),
            );
            if nxt > 0 {
                self.edges[id].flow += nxt;
                self.edges[id ^ 1].flow -= nxt;
                return nxt;
            }
            self.ptr[v] += 1;
        }
        0
    }
    pub fn matching(&mut self, ret: MatchingReturnValue) -> Option<Vec<Vec<usize>>> {
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
        match ret {
            MatchingReturnValue::MatchingGraph => {
                let mut ans = vec![Vec::<usize>::new(); self.graph.len() - 2];
                for v in 0..self.graph.len() - 2 - self.vals.len() {
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
            MatchingReturnValue::FlowGraph => {
                let mut ans = vec![Vec::<usize>::new(); self.graph.len()];
                for (v, edges) in self.graph.iter().enumerate() {
                    for id in edges.iter().cloned() {
                        let u = self.edges[id].to;
                        if self.edges[id].capacity > self.edges[id].flow {
                            ans[v].push(u);
                        }
                        if self.edges[id].flow > 0 {
                            ans[u].push(v);
                        }
                    }
                }
                Some(ans)
            }
        }
    }
}

pub struct AllDifferentACPropagator {
    pcb: PropagatorControlBlock,
    vars: Vec<Rc<RefCell<Variable>>>,
}

impl AllDifferentACPropagator {
    pub fn new(vars: Vec<Rc<RefCell<Variable>>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            vars,
        }
    }
}

impl Propagator for AllDifferentACPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for v in &self.vars {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for v in &self.vars {
            v.borrow_mut()
                .remove_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(&mut self, _search: &mut Search<'_>) -> PropagatorState {
        let mut m = ACMatching::new(&self.vars, None);
        if let Some(g) = m.matching(MatchingReturnValue::MatchingGraph) {
            let mut scc = SCC::new(g);
            let mut edges = scc.get_bad_edges();
            for (val, i) in edges.drain(..) {
                self.vars[i]
                    .borrow_mut()
                    .remove(m.vals[val - self.vars.len()]);
            }
        } else {
            self.vars[0].borrow().fail();
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
