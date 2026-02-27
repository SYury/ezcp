use crate::propagator::Propagator;
use crate::search::Search;
use std::cell::RefCell;
use std::rc::Rc;

pub trait Constraint {
    /// True if all variables are assigned and the constraint is satisfied.
    fn satisfied(&self) -> bool;
    /// True if the constraint is definitely unsatisfied (may return true even if not all variables are assigned).
    fn failed(&self) -> bool;
    /// Creates propagators with indices starting from index0
    fn create_propagators(&self, index0: usize) -> Vec<Rc<RefCell<dyn Propagator>>>;
    fn add_propagators(&self, search: &mut Search<'_>) {
        let prop = self.create_propagators(search.get_propagator_id());
        search.advance_propagator_id(prop.len());
        for p in prop {
            search.add_propagator(p.clone());
            p.borrow().listen(p.clone());
        }
    }
}
