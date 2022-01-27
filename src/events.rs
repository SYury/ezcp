#[derive(Copy, Clone)]
pub enum Event {
    Modified = 0,
    LowerBound = 1,
    UpperBound = 2,
    Assigned = 3,
}

pub const N_EVENTS: usize = 4;

pub fn event_index(e: &Event) -> usize {
    *e as usize
}
