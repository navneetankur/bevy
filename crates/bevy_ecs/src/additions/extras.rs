use core::any::Any;

pub struct Extras {
    pub packet_systems: Vec<Option<Box<dyn Any>>>,
    pub next_packet_id: usize,
}

impl Default for Extras {
    fn default() -> Self {
        Self {
            packet_systems: Vec::with_capacity(100),
            next_packet_id: 0,
        }
    }
}
