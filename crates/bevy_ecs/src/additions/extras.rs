use core::any::Any;

#[derive(Default)]
pub struct Extras {
    pub event_systems: Vec<Option<Box<dyn Any>>>,
    pub next_event_index: usize,
}
