use core::any::Any;

use crate::world::CommandQueue;

pub struct Extras {
    pub packet_systems: Vec<Option<Box<dyn Any>>>,
    pub next_packet_id: usize,
    pub queue: Option<CommandQueue>,
}

impl Default for Extras {
    fn default() -> Self {
        Self {
            packet_systems: Vec::with_capacity(100),
            next_packet_id: 0,
            queue: Some(Default::default()),
        }
    }
}
