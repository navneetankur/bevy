use derive_more::derive::{Deref, DerefMut};
use crate::world::{CommandQueue, World};
use super::OptionPacket;

#[derive(Default, Deref, DerefMut)]
pub struct ECommands {
    queue: CommandQueue,
}
impl OptionPacket for ECommands {
    fn run(mut self, world: &mut World) {
        self.apply(world);
    }
}
