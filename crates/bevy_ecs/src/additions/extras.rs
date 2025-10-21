use core::any::Any;
use std::boxed::Box;

use bevy_utils::TypeIdMap;

pub struct Extras {
    // pub packet_systems: Vec<Option<Box<dyn Any>>>,
    pub packet_systems: TypeIdMap<Box<dyn Any>>,
}

impl Default for Extras {
    fn default() -> Self {
        Self {
            packet_systems: TypeIdMap::with_capacity_and_hasher(100, Default::default()),
        }
    }
}
