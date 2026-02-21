use crate::constraint::Constraint;
use crate::events::Event;
use crate::propagator::{Propagator, PropagatorControlBlock};
use crate::search::Search;
use crate::variable::Variable;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub struct ArrayIntElementConstraint {
    index: Rc<RefCell<Variable>>,
    value: Rc<RefCell<Variable>>,
    array: Vec<i64>,
}

impl ArrayIntElementConstraint {
    pub fn new(
        index: Rc<RefCell<Variable>>,
        value: Rc<RefCell<Variable>>,
        array: Vec<i64>,
    ) -> Self {
        if array.is_empty() {
            panic!("ArrayIntElementConstraint: empty array is not allowed.");
        }
        Self {
            index,
            value,
            array,
        }
    }
}

impl Constraint for ArrayIntElementConstraint {
    fn satisfied(&self) -> bool {
        let i = self.index.borrow();
        let v = self.value.borrow();
        if !i.is_assigned() || !v.is_assigned() {
            return false;
        }
        let pos = i.value();
        if pos < 1 || pos > (self.array.len() as i64) {
            return false;
        }
        self.array[pos as usize - 1] == v.value()
    }
    fn create_propagators(&self, search: &mut Search<'_>) {
        let p = Rc::new(RefCell::new(ArrayIntElementACPropagator::new(
            self.index.clone(),
            self.value.clone(),
            self.array.clone(),
            search.new_propagator_id(),
        )));
        search.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

pub struct ArrayIntElementACPropagator {
    pcb: PropagatorControlBlock,
    index: Rc<RefCell<Variable>>,
    value: Rc<RefCell<Variable>>,
    array: Vec<i64>,
}

impl ArrayIntElementACPropagator {
    pub fn new(
        index: Rc<RefCell<Variable>>,
        value: Rc<RefCell<Variable>>,
        array: Vec<i64>,
        id: usize,
    ) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            index,
            value,
            array,
        }
    }
}

impl Propagator for ArrayIntElementACPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.index
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        self.value
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Assigned);
    }

    fn propagate(&mut self) {
        let mut idx = self.index.borrow_mut();
        idx.set_lb(1);
        idx.set_ub(self.array.len() as i64);
        let mut possible = HashSet::new();
        for v in idx.iter() {
            possible.insert(self.array[v as usize - 1]);
        }
        let mut val = self.value.borrow_mut();
        let mut remove = HashSet::new();
        for v in val.iter() {
            if !possible.contains(&v) {
                remove.insert(v);
            }
        }
        for v in remove.into_iter() {
            val.remove(v);
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

pub struct ArrayVarElementConstraint {
    index: Rc<RefCell<Variable>>,
    value: Rc<RefCell<Variable>>,
    array: Vec<Rc<RefCell<Variable>>>,
}

impl ArrayVarElementConstraint {
    pub fn new(
        index: Rc<RefCell<Variable>>,
        value: Rc<RefCell<Variable>>,
        array: Vec<Rc<RefCell<Variable>>>,
    ) -> Self {
        if array.is_empty() {
            panic!("ArrayVarElementConstraint: empty array is not allowed.");
        }
        Self {
            index,
            value,
            array,
        }
    }
}

impl Constraint for ArrayVarElementConstraint {
    fn satisfied(&self) -> bool {
        let i = self.index.borrow();
        let v = self.value.borrow();
        if !i.is_assigned() || !v.is_assigned() {
            return false;
        }
        let pos = i.value();
        if pos < 1 || pos > (self.array.len() as i64) {
            return false;
        }
        let elem = self.array[pos as usize - 1].borrow();
        elem.is_assigned() && elem.value() == v.value()
    }
    fn create_propagators(&self, search: &mut Search) {
        let p = Rc::new(RefCell::new(ArrayVarElementACPropagator::new(
            self.index.clone(),
            self.value.clone(),
            self.array.clone(),
            search.new_propagator_id(),
        )));
        search.add_propagator(p.clone());
        p.borrow().listen(p.clone());
    }
}

pub struct ArrayVarElementACPropagator {
    pcb: PropagatorControlBlock,
    index: Rc<RefCell<Variable>>,
    value: Rc<RefCell<Variable>>,
    array: Vec<Rc<RefCell<Variable>>>,
}

impl ArrayVarElementACPropagator {
    pub fn new(
        index: Rc<RefCell<Variable>>,
        value: Rc<RefCell<Variable>>,
        array: Vec<Rc<RefCell<Variable>>>,
        id: usize,
    ) -> Self {
        Self {
            pcb: PropagatorControlBlock::new(id),
            index,
            value,
            array,
        }
    }
}

impl Propagator for ArrayVarElementACPropagator {
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>) {
        self.index
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Modified);
        self.value
            .borrow_mut()
            .add_listener(self_pointer.clone(), Event::Assigned);
        for v in &self.array {
            v.borrow_mut()
                .add_listener(self_pointer.clone(), Event::Modified);
        }
    }

    fn propagate(&mut self) {
        let mut idx = self.index.borrow_mut();
        idx.set_lb(1);
        idx.set_ub(self.array.len() as i64);
        let mut possible = HashSet::new();
        for v in idx.iter() {
            for u in self.array[v as usize - 1].borrow().iter() {
                possible.insert(u);
            }
        }
        let mut val = self.value.borrow_mut();
        let mut remove = HashSet::new();
        for v in val.iter() {
            if !possible.contains(&v) {
                remove.insert(v);
            }
        }
        for v in remove.into_iter() {
            val.remove(v);
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
