#[derive(Copy, Clone)]
pub enum Event {
    Modified = 0,
    LowerBound = 1,
    UpperBound = 2,
    Assigned = 3,
    Removed = 4,
}

pub const N_EVENTS: usize = 5;

pub fn event_index(e: &Event) -> usize {
    *e as usize
}
