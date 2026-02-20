/* This program finds any spanning tree of the graph with max degree <= k.
 * With k = 2 the problem becomes Hamiltonian path problem.
 *
 * Input format:
 * #vertices #edges
 * [edge list]
 * maximum degree
 *
 * Output format:
 * [edge list] or infeasibility message
 */
use ezcp::brancher::MinValueBrancher;
use ezcp::config::Config;
use ezcp::constraint::Constraint;
use ezcp::events::Event;
use ezcp::gcc::GlobalCardinalityACPropagator;
use ezcp::graph::TreeConstraint;
use ezcp::propagator::{Propagator, PropagatorControlBlock};
use ezcp::solver::{SolutionStatus, Solver};
use ezcp::variable::Variable;
use ezcp::variable_selector::FirstFailVariableSelector;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
struct Scanner {
    buffer: Vec<String>
}

impl Scanner {
    fn next<T: std::str::FromStr>(&mut self) -> T {
        loop {
            if let Some(token) = self.buffer.pop() {
                return token.parse().ok().expect("Failed parse");
            }
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).expect("Failed read");
            self.buffer = input.split_whitespace().rev().map(String::from).collect();
        }
    }
}

fn read_graph(scanner: &mut Scanner) -> Vec<Vec<usize>> {
    let vertices = scanner.next::<usize>();
    let edges = scanner.next::<usize>();
    let mut graph = vec![Vec::new(); vertices];
    for _ in 0..edges {
        let v = scanner.next::<usize>() - 1;
        let u = scanner.next::<usize>() - 1;
        graph[v].push(u);
        graph[u].push(v);
    }
    graph
}

struct DegreeConstraint {
    max_degree: usize,
    parent: Vec<Rc<RefCell<Variable>>>,
}

impl DegreeConstraint {
    pub fn new(max_degree: usize, parent: Vec<Rc<RefCell<Variable>>>) -> Self {
        Self {
            max_degree,
            parent
        }
    }
}

impl Constraint for DegreeConstraint {
    fn satisfied(&self) -> bool {
        let mut deg = vec![0; self.parent.len()];
        for (v, var) in self.parent.iter().enumerate() {
            if !var.borrow().is_assigned() {
                return false;
            }
            let p = var.borrow().value() as usize;
            if v != p {
                deg[v] += 1;
                deg[p] += 1;
            }
        }
        *deg.iter().max().unwrap() <= self.max_degree
    }
    fn create_propagators(&self, solver: &mut Solver) {
        let p = Rc::new(RefCell::new(DegreePropagator::new(
            self.max_degree,
            self.parent.clone(),
            solver.new_propagator_id(),
        )));
        solver.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

struct DegreePropagator {
    pcb: PropagatorControlBlock,
    max_degree: i32,
    parent: Vec<Rc<RefCell<Variable>>>,
}

impl DegreePropagator {
    pub fn new(max_degree: usize, parent: Vec<Rc<RefCell<Variable>>>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            max_degree: max_degree as i32,
            parent,
        }
    }
}

impl Propagator for DegreePropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for v in &self.parent {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(&mut self) {
        let mut card = HashMap::<i64, i32>::new();
        for (i, v) in self.parent.iter().enumerate() {
            if v.borrow().possible(i as i64) {
                card.insert(i as i64, self.max_degree + 1);
            } else {
                card.insert(i as i64, self.max_degree - 1);
            }
        }
        GlobalCardinalityACPropagator::new(self.parent.clone(), card, 0).propagate();
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

fn main() {
    let mut scanner = Scanner::default();
    let g = read_graph(&mut scanner);
    let n = g.len();
    let max_degree = scanner.next::<usize>();
    let mut solver = Solver::new(
        Config::new(
            Box::new(MinValueBrancher {}),
            Box::new(FirstFailVariableSelector {}),
        )
    );
    let ntree = solver.new_variable(1, 1, format!("ntree"));
    let mut parent = Vec::with_capacity(n);
    for i in 0..n {
        parent.push(solver.new_variable(0, (n as i64) - 1, format!("parent_{}", i)));
    }
    for v in 0..n {
        let mut ptr = 0;
        for u in g[v].iter().cloned() {
            while ptr < u {
                if ptr != v {
                    parent[v].borrow_mut().remove(ptr as i64);
                }
                ptr += 1;
            }
            ptr += 1;
        }
    }
    solver.add_constraint(Box::new(TreeConstraint::new(
                ntree,
                parent.clone(),
    )));
    solver.add_constraint(Box::new(DegreeConstraint::new(
                max_degree,
                parent.clone(),
                )));
    if solver.solve() == SolutionStatus::Infeasible {
        println!("No spanning tree with degree <= {} found.", max_degree);
    } else {
        let mut root = n;
        for v in 0..n {
            let u = parent[v].borrow().value() as usize;
            if v != u {
                println!("{} {}", v + 1, u + 1);
            } else {
                assert!(root == n);
                root = v;
            }
        }
        assert!(root < n);
    }
}
