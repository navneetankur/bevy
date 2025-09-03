use core::any::Any;
use std::{boxed::Box, vec::Vec};

pub struct Extras {
    pub packet_systems: Vec<Option<Box<dyn Any>>>,
}

impl Default for Extras {
    fn default() -> Self {
        Self {
            packet_systems: Vec::with_capacity(100),
        }
    }
}
