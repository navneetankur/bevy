pub mod ecommands;
pub mod packetsystem;
pub mod exclusivepacketsystem;
pub mod packetslicer;
mod optionpacket;
pub use optionpacket::OptionPacket;
use core::any::{type_name, TypeId};
use std::boxed::Box;
use std::vec::Vec;
use crate::{self as bevy_ecs, system::Commands};
pub use crate::system::SystemInput;
pub use bevy_ecs_macros::Packet;
pub use bevy_ecs_macros::SmolId;
use packetsystem::IntoPacketSystem;
use crate::system::{System};
use crate::resource::Resource;
use smallvec::SmallVec;
use crate::{system::BoxedSystem, world::World};

pub unsafe fn next_packet_id(world: &mut World) -> usize {
    static mut NEXT_PACKET_ID: usize = 0;
    let rv = NEXT_PACKET_ID;
    // 0 for E, 1 for &E, w for &[E]
    NEXT_PACKET_ID += 3;
    return rv;
}

pub struct PacketInSystem<E: SystemInput> {
    pub v: BoxedSystem<E, ()>,
    pub tid: TypeId,
}
pub struct RegisteredSystems<E: SystemInput>{
    pub v: SmallVec<[PacketInSystem<E>; 1]>,
}
// pub trait IRegisteredSystem {
//     fn as_registered_systems<E: SystemInput>(&mut self) -> &mut RegisteredSystems<E>;
// }
// impl<E: SystemInput> IRegisteredSystem for RegisteredSystems<E>{
//     fn as_registered_systems<F: SystemInput>(&mut self) -> &mut RegisteredSystems<F> {self}
// }

pub trait Packet: SystemInput + SmolId + 'static { }
pub trait SmolId { fn sid(world: &mut World) -> usize; }

pub fn register_system<I, Out, F, M>(world: &mut World, f: F)
where
    I: SystemInput + SmolId + 'static,
    Out: OptionPacket,
    F: IntoPacketSystem<I, Out, M> + 'static,
    M: 'static,
{
    #[cfg(debug_assertions)]
    world.init_resource::<PacketInMotion>();
    // don't forget to put it back.
    let mut systems = world.remove_packet_system::<I>().unwrap_or_default();

    let tid = TypeId::of::<F>();
    #[cfg(debug_assertions)]
    {
        for system in &systems.v {
            assert_ne!(system.tid, tid);
        }
    }
    let mut system = IntoPacketSystem::into_system(f);
    system.initialize(world);
    let system = PacketInSystem { v: Box::new(system), tid };
    systems.v.push(system);

    // put back here.
    world.put_back_packet_system(systems);
}
pub fn unregister_system<I, Out, F, M>(world: &mut World, _: F)
where
    I: SystemInput + SmolId + 'static,
    Out: OptionPacket,
    F: IntoPacketSystem<I, Out, M> + 'static,
    M: 'static,
{
    world.with_packet_system::<I>(|_, systems| {
        let tid = TypeId::of::<F>();
        systems.v.retain(|s| s.tid != tid);
    });
}
#[derive(Resource, Default)]
struct PacketInMotion(Vec<TypeId>);
pub fn run_this_packet_system<'a, const SLICE: bool, E>(event: E, world: &mut World)
where 
    E: Packet,
    for<'d> E: SystemInput<Inner<'d> = E>,
    for<'b> &'b E: SmolId,
    for<'c> &'c [E]: SmolId,
{
    #[cfg(debug_assertions)]
    {
        let mut in_motion = world.resource_mut::<PacketInMotion>();
        if in_motion.0.contains(&TypeId::of::<E>()) {
            panic!("Recursive event {:?}", type_name::<E>());
        } else {
            in_motion.0.push(TypeId::of::<E>());
        }
    }
    run_for_ref_packet(world, &event);
    if SLICE { run_for_single_slice_packet(world, &event); }
    run_for_val_packet(world, event);
    #[cfg(debug_assertions)]
    {
        let mut in_motion = world.resource_mut::<PacketInMotion>();
        let index = in_motion.0.iter().position(|x| *x == TypeId::of::<E>()).unwrap();
        in_motion.0.swap_remove(index);
    }
}

fn run_for_val_packet<E>(world: &mut World, event: E)
where
    E: Packet,
    E: for<'e> SystemInput<Inner<'e> = E>
{
    world.with_packet_system::<E>(|world, systems| {
        let mut systems_iter = systems.v.iter_mut();
        let Some(system) = systems_iter.next() else { return };
        system.v.run(event, world);
        debug_assert!(systems_iter.len() == 0, "Only one system can take value {:?}", type_name::<E>());
    });
}

fn run_for_ref_packet<E>(world: &mut World, event: &E)
where
    E: Packet,
    for<'a> &'a E: SmolId,
    // E: SystemInput<Inner<'static> = E>
{
    world.with_packet_system::<&E>(|world, systems| {
        for system in &mut systems.v {
            system.v.run(event, world);
        }
    });
}
fn run_for_single_slice_packet<E>(world: &mut World, event: &E)
where
    E: Packet,
    for<'a> &'a [E]: SmolId,
    // E: SystemInput<Inner<'static> = E>
{
    //don't forget to put it back.
    // let Some(mut systems) = world.remove_event_system::<&[E]>() else {return;};
    world.with_packet_system::<&[E]>(|world, systems| {
        let event_slice = core::slice::from_ref(event);
        for system in &mut systems.v {
            system.v.run(event_slice, world);
        }
    });
    // put back
    // world.put_back_event_system(systems);
}
impl<E: SystemInput> Default for RegisteredSystems<E> {
    fn default() -> Self {
        RegisteredSystems { v: Default::default()}
    }
}
pub trait Eventy: SystemInput{}

impl World {
    pub fn send<'a,'b,E>(&mut self, packet: E)
    where
        E: Packet,
        E: for<'e> SystemInput<Inner<'e> = E>,
        for<'d> &'d E: SmolId,
        for<'c> &'c [E]: SmolId,
    {
        run_this_packet_system::<true, E>(packet, self);
    }

    pub fn register_packet_system<I, Out, F, M>(&mut self, f: F)
    where
        I: SystemInput + SmolId+ 'static,
        Out: OptionPacket,
        F: IntoPacketSystem<I,Out, M> + 'static,
        M: 'static,
    {
        register_system(self, f);
    }
    pub fn unregister_packet_system<I, Out, F, M>(&mut self, f: F)
    where
        I: SystemInput + SmolId+ 'static,
        Out: OptionPacket,
        F: IntoPacketSystem<I,Out, M> + 'static,
        M: 'static,
    {
        unregister_system(self, f);
    }

    fn with_packet_system<I>(&mut self, f: impl FnOnce(&mut World, &mut RegisteredSystems<I>),)
    where 
        I: SystemInput + SmolId + 'static,
    {
        let Some(mut systems) = self.remove_packet_system::<I>() else {return};
        f(self, &mut systems);
        self.put_back_packet_system(systems);
    }

    /// don't forget to put it back.
    fn remove_packet_system<I: SystemInput + SmolId + 'static>(&mut self) -> Option<Box<RegisteredSystems<I>>> {
        let event_index = I::sid(self);
        let event_systems = &mut self.extras.packet_systems;
        if event_systems.len() <= event_index {
            event_systems.resize_with(event_index + 1, || None);
            return None;
        }
        let rv = event_systems[event_index].take();
        return rv.map(|v| v.downcast().unwrap());
    }

    fn put_back_packet_system<I: SystemInput + SmolId + 'static>(&mut self, systems: Box<RegisteredSystems<I>>) {
        let event_index = I::sid(self);
        let event_systems = &mut self.extras.packet_systems;
        debug_assert!(event_systems[event_index].is_none());
        event_systems[event_index] = Some(systems);
    }

}
impl<'w,'s> Commands<'w,'s> {
    pub fn send<E>(&mut self, packet: E)
    where
        E: Packet,
        for<'e> E: SystemInput<Inner<'e> = E>,
        for<'d> &'d E: SmolId,
        for<'c> &'c [E]: SmolId,
    {
        self.queue(move |world: &mut World| world.send(packet));
    }
}
impl<E: Packet> SystemInput for &E {
    type Param<'i> = &'i E;
    type Inner<'i> = &'i E;
    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}
impl<E: Packet + SmolId> SmolId for &E {
    fn sid(world: &mut World) -> usize { E::sid(world) + 1 }
}
impl<E: Packet + SmolId> SmolId for &[E] {
    fn sid(world: &mut World) -> usize { E::sid(world) + 2 }
}


#[cfg(test)]
mod tests {
    use crate::{system::ResMut, world::World};
    use super::Packet;
    use crate::resource::Resource;

    
    #[derive(Resource)]
    struct Count(u8);

    #[derive(Packet)]
    struct Input(u8);

    #[derive(Packet)]
    struct Moved;

    #[test]
    fn test() {
        let mut world = World::new();
        world.insert_resource(Count(0));
        world.register_packet_system(move_player);
        world.register_packet_system(count_moved);
        world.register_packet_system(count_moved1);
        world.register_packet_system(count_moved2);
        world.send(Input(b'a'));
        let count = world.get_resource::<Count>().unwrap();
        assert_eq!(count.0, 3);
    }

    fn move_player(Input(input): Input) -> Option<Moved> {
        match input {
            b'a' => Some(Moved),
            _ => None
        }
    }

    fn count_moved1(_: &Moved, mut count: ResMut<Count>) {
        count.0 += 1;
    }
    fn count_moved2(_: &Moved, mut count: ResMut<Count>) {
        count.0 += 1;
    }
    fn count_moved(_: Moved, mut count: ResMut<Count>) {
        count.0 += 1;
    }

}
