use crate::alldifferent::{ACMatching, MatchingReturnValue};
use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock};
use crate::scc::compute_scc;
use crate::search::Search;
use crate::variable::Variable;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct GlobalCardinalityConstraint {
    vars: Vec<Rc<RefCell<Variable>>>,
    card: HashMap<i64, i32>,
}

impl GlobalCardinalityConstraint {
    pub fn new(vars: Vec<Rc<RefCell<Variable>>>, card: HashMap<i64, usize>) -> Self {
        let mut c = HashMap::new();
        for (k, v) in card {
            c.insert(k, v as i32);
        }
        Self { vars, card: c }
    }
}

impl Constraint for GlobalCardinalityConstraint {
    fn satisfied(&self) -> bool {
        let mut card = HashMap::<i64, i32>::new();
        for v in &self.vars {
            if !v.borrow().is_assigned() {
                return false;
            }
            let val = v.borrow().value();
            if let Some(c) = card.get_mut(&val) {
                *c += 1;
            } else {
                card.insert(val, 1);
            }
        }
        for (k, v) in card.drain() {
            if let Some(c) = self.card.get(&k) {
                if *c < v {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
    fn create_propagators(&self, search: &mut Search<'_>) {
        let p = Rc::new(RefCell::new(GlobalCardinalityACPropagator::new(
            self.vars.clone(),
            self.card.clone(),
            search.new_propagator_id(),
        )));
        search.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

pub struct GlobalCardinalityACPropagator {
    pcb: PropagatorControlBlock,
    vars: Vec<Rc<RefCell<Variable>>>,
    card: HashMap<i64, i32>,
}

impl GlobalCardinalityACPropagator {
    pub fn new(vars: Vec<Rc<RefCell<Variable>>>, card: HashMap<i64, i32>, id: usize) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            vars,
            card,
        }
    }
}

impl Propagator for GlobalCardinalityACPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        for v in &self.vars {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(&mut self) {
        let mut m = ACMatching::new(&self.vars, Some(&self.card));
        if let Some(g) = m.matching(MatchingReturnValue::FlowGraph) {
            let scc = compute_scc(&g);
            let mut comp_id = vec![0; g.len()];
            for (i, comp) in scc.iter().enumerate() {
                for v in comp.iter().cloned() {
                    comp_id[v] = i;
                }
            }
            for v in 0..g.len() {
                for u in g[v].iter().cloned() {
                    if v >= g.len() - 2 || u >= g.len() - 2 {
                        continue;
                    }
                    if v < self.vars.len() && v < u && comp_id[v] != comp_id[u] {
                        self.vars[v]
                            .borrow_mut()
                            .remove(m.vals[u - self.vars.len()]);
                    }
                }
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

    fn is_idempotent(&self) -> bool {
        true
    }
}
