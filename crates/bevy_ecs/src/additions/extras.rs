use core::any::Any;

#[derive(Default)]
pub struct Extras {
    pub packet_systems: Vec<Option<Box<dyn Any>>>,
}
