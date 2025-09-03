use core::ops::{Deref, DerefMut};
use std::vec::Vec;

use bevy_utils::synccell::SyncCell;
use derive_more::derive::{Deref, DerefMut};

use crate::{system::{ReadOnlySystemParam, SystemParam}, world::World};

use super::{run_this_packet_system, OptionPacket, Packet, SystemInput};

pub struct PacketSlicer<'s, E: Packet>(&'s mut Vec<E>);
impl<'s, E: Packet> PacketSlicer<'s, E> {
    fn new(v: &'s mut Vec<E>) -> Self { Self(v) }
}
impl<E: Packet> Deref for PacketSlicer<'_, E> {
    type Target = Vec<E>;
    fn deref(&self) -> &Self::Target { self.0 }
}
impl<E: Packet> DerefMut for PacketSlicer<'_, E> {
    fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}

unsafe impl<E: Packet + 'static> SystemParam for PacketSlicer<'_, E>
where 
    for<'e> E: SystemInput<Inner<'e> = E>,
{
    type State = SyncCell<Vec<E>>;

    type Item<'world, 'state> = PacketSlicer<'state, E>;

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
       for event in state.get().drain(..) {
           // todo optimization: get the systems and run one it
           // instead of getting event multiple times and hitting hashmap in world.
           run_this_packet_system::<false, E>(event, world);
       }
    }
}
unsafe impl<E: Packet + 'static> ReadOnlySystemParam for PacketSlicer<'_, E>
where 
    for<'e> E: SystemInput<Inner<'e> = E>,
{}

impl<E: Packet> SystemInput for &[E] {
    type Param<'i> = &'i [E];
    type Inner<'i> = &'i [E];
    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> { this }
}
pub fn run_for_slice_packet<E>(world: &mut World, event_slice: &[E])
where
    E: Packet,
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
    for<'e> P: SystemInput<Inner<'e> = P>,
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

