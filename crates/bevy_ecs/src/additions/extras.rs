use core::any::Any;

use crate::world::command_queue::RawCommandQueue;

pub struct Extras {
    pub packet_systems: Vec<Option<Box<dyn Any>>>,
    pub next_packet_id: usize,
    pub queue: RawCommandQueue,
}

impl Default for Extras {
    fn default() -> Self {
        Self {
            packet_systems: Vec::with_capacity(100),
            next_packet_id: 0,
            queue: RawCommandQueue::new(),
        }
    }
}
