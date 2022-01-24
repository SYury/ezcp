use std::cell::RefCell;
use std::rc::Rc;
use crate::variable::Variable;

pub struct PropagatorControlBlock {
    pub has_new_events: bool,
    pub queued: bool,
    pub id: usize,
}

pub trait Propagator {

    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>);

    fn propagate(&mut self);

    fn get_cb(&self) -> &PropagatorControlBlock;

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock;

    fn is_idemponent(&self) -> bool {
        false
    }

    fn new_event(&mut self) {
        self.get_cb_mut().has_new_events = true;
    }

    fn has_new_events(&self) -> bool {
        self.get_cb().has_new_events
    }

    fn clear_events(&mut self) {
        self.get_cb_mut().has_new_events = false;
    }

    fn enqueue(&mut self) {
        self.get_cb_mut().queued = true;
    }

    fn dequeue(&mut self) {
        self.get_cb_mut().queued = false;
    }

    fn is_queued(&self) -> bool {
        self.get_cb().queued
    }

    fn get_id(&self) -> usize {
        self.get_cb().id
    }
}

