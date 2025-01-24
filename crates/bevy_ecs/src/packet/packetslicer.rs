use core::ops::{Deref, DerefMut};

use bevy_utils::synccell::SyncCell;
use derive_more::derive::{Deref, DerefMut};

use crate::{system::{ReadOnlySystemParam, SystemParam}, world::World};

use super::{run_this_packet_system, OptionPacket, Packet, SmolId, SystemInput};

pub struct PacketSlicer<'s, E: Packet, const FORWARD: bool = true>(&'s mut Vec<E>);
impl<'s, E: Packet, const F: bool> PacketSlicer<'s, E, F> {
    fn new(v: &'s mut Vec<E>) -> Self { Self(v) }
}
impl<E: Packet, const FORWARD: bool> Deref for PacketSlicer<'_, E, FORWARD> {
    type Target = Vec<E>;
    fn deref(&self) -> &Self::Target { self.0 }
}
impl<E: Packet, const FORWARD: bool> DerefMut for PacketSlicer<'_, E, FORWARD> {
    fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}

unsafe impl<E: Packet, const FORWARD: bool> SystemParam for PacketSlicer<'_, E, FORWARD>
where 
    E: SystemInput<Inner<'static> = E>,
    for<'b> &'b E: SmolId,
    for<'c> &'c [E]: SmolId,
{
    type State = SyncCell<Vec<E>>;

    type Item<'world, 'state> = PacketSlicer<'state, E, FORWARD>;

    fn init_state(_: &mut World, _: &mut crate::system::SystemMeta) -> Self::State {
        SyncCell::new(Vec::with_capacity(2))
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        _: &crate::system::SystemMeta,
        _: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        _: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        PacketSlicer::new(state.get())
    }

    fn apply(state: &mut Self::State, _: &crate::system::SystemMeta, world: &mut World) {
        run_for_slice_packet(world, state.get());
        if FORWARD {
           for event in state.get().drain(..) {
               // todo optimization: get the systems and run one it
               // instead of getting event multiple times and hitting hashmap in world.
               run_this_packet_system::<false, E>(event, world);
           }
        }
        else {
            state.get().clear();
        }
    }
}
unsafe impl<E: Packet, const FORWARD: bool> ReadOnlySystemParam for PacketSlicer<'_, E, FORWARD>
where 
    E: SystemInput<Inner<'static> = E>,
    for<'b> &'b E: SmolId,
    for<'c> &'c [E]: SmolId,
{}

impl<E: Packet> SystemInput for &[E] {
    type Param<'i> = &'i [E];
    type Inner<'i> = &'i [E];
    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> { this }
}
pub fn run_for_slice_packet<E>(world: &mut World, event_slice: &[E])
where
    E: Packet,
    for<'b> &'b E: SmolId,
    for<'c> &'c [E]: SmolId,
{
    //don't forget to put it back.
    let Some(mut systems) = world.remove_packet_system::<&[E]>() else {return;};
    for system in &mut systems.v {
        system.v.run(event_slice, world);
    }
    //put back
    world.put_back_packet_system(systems);
}

#[derive(Deref, DerefMut, Default)]
pub struct PacketVec<P: Packet>(Vec<P>);

impl<P: Packet> OptionPacket for PacketVec<P>
where 
    P: SystemInput<Inner<'static> = P>,
{
    fn run(self, world: &mut World) {
        run_for_slice_packet(world, &self.0);
        for packet in self.0 {
           // todo optimization: get the systems and run one it
           // instead of getting event multiple times and hitting hashmap in world.
           run_this_packet_system::<false, P>(packet, world);
        }
    }
}

