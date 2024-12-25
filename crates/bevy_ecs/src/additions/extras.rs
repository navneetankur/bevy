use core::any::Any;

use bevy_utils::TypeIdMap;

#[derive(Default)]
pub struct Extras {
    pub packet_systems: Vec<Option<Box<dyn Any>>>,
}
