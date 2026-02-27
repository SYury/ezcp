use crate::search::Search;
use std::cell::RefCell;
use std::rc::Rc;

pub struct PropagatorControlBlock {
    pub has_new_events: bool,
    pub queued: bool,
    pub id: usize,
}

impl PropagatorControlBlock {
    pub fn new(id: usize) -> Self {
        Self {
            has_new_events: false,
            queued: false,
            id,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PropagatorState {
    Normal,
    /// Tells the search to remove the propagator in the current subtree.
    Terminated,
}

/// This enum is to be used by constraints with more than one possible propagator.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PropagationLevel {
    AC,
    BC,
}

pub trait Propagator {
    /// Subscribes itself to all required events.
    /// Important note: self_pointer may actually not point to self (this happens with reified propagators).
    fn listen(&self, self_pointer: Rc<RefCell<dyn Propagator>>);

    /// Unsubscribes itself from all required events.
    fn unlisten(&self, self_pointer: Rc<RefCell<dyn Propagator>>);

    fn propagate(&mut self, search: &mut Search<'_>) -> PropagatorState;

    fn get_cb(&self) -> &PropagatorControlBlock;

    fn get_cb_mut(&mut self) -> &mut PropagatorControlBlock;

    fn is_idempotent(&self) -> bool {
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
