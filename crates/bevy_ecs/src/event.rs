pub mod eventsystem;
use core::any::{type_name, TypeId};
use crate::{self as bevy_ecs};
pub use crate::system::SystemInput;
pub use bevy_ecs_macros::Event;
use eventsystem::IntoEventSystem;
use crate::{system::{Resource, System}, world::Mut};
use bevy_utils::TypeIdMap;
use smallvec::SmallVec;
use crate::{system::BoxedSystem, world::World};

pub struct EventInSystem<E: SystemInput> {
    pub v: BoxedSystem<E, Box<dyn Event>>,
    pub tid: TypeId,
}
#[derive(Resource)]
pub struct RegisteredSystems<E: SystemInput>{
    pub v: SmallVec<[EventInSystem<E>; 1]>,
    pub tid: TypeIdMap<usize>,
}

pub trait Event: Send + Sync + 'static {
    fn run_systems(self: Box<Self>, world: &mut World);
}
impl Event for (){ fn run_systems(self: Box<()>, _: &mut World) {} }
pub fn register_system<I, Out, F, M>(world: &mut World, f: F)
where
    I: SystemInput + 'static,
    Out: Event,
    F: IntoEventSystem<I, Out, M> + 'static,
    M: 'static,
{
    world.init_resource::<RegisteredSystems<I>>();
    world.resource_scope(|world: &mut World, mut systems: Mut<RegisteredSystems<I>>| {
        let tid = TypeId::of::<F>();
        if systems.tid.contains_key(&tid) { return };
        let mut system = IntoEventSystem::into_system(f);
        system.initialize(world);
        let system = EventInSystem { v: Box::new(system), tid };
        systems.v.push(system);
        let index = systems.v.len() - 1;
        systems.tid.insert(tid, index);
    });
}
pub fn run_this_event_system<'a, E>(event: E, world: &mut World)
where 
    E: Event,
    E: SystemInput<Inner<'static> = E>,
{
    run_for_ref_event(world, &event);

     // don't forget to put it back.
     let Some(mut systems) = world.remove_resource::<RegisteredSystems<E>>() else {return};
     let mut systems_iter = systems.v.iter_mut();
     let Some(system) = systems_iter.next() else { return };
     let new_event = system.v.run(event, world);
     new_event.run_systems(world);
     debug_assert!(systems_iter.next().is_none(), "Only one system can take value {:?}", type_name::<E>());
     debug_assert!(!world.contains_resource::<RegisteredSystems<E>>());
     world.insert_resource(systems);
}
pub fn run_this_boxed_event_system<'a, E>(event: Box<E>, world: &mut World)
where 
    E: Event,
    E: SystemInput<Inner<'static> = E>,
{
    run_for_ref_event(world, event.as_ref());

     // don't forget to put it back.
     let Some(mut systems) = world.remove_resource::<RegisteredSystems<E>>() else {return};
     let mut systems_iter = systems.v.iter_mut();
     let Some(system) = systems_iter.next() else { return };
     let new_event = system.v.run(*event, world);
     new_event.run_systems(world);
     debug_assert!(systems_iter.next().is_none(), "Only one system can take value {:?}", type_name::<E>());
     debug_assert!(!world.contains_resource::<RegisteredSystems<E>>());
     world.insert_resource(systems);
}

fn run_for_ref_event<E>(world: &mut World, event: &E) where E: Event, E: SystemInput<Inner<'static> = E> {
    //don't forget to put it back.
    let Some(mut systems) = world.remove_resource::<RegisteredSystems<&E>>() else {return};
    for system in &mut systems.v {
        let new_event = system.v.run(event, world);
        new_event.run_systems(world);
    }
    debug_assert!(!world.contains_resource::<RegisteredSystems<&E>>());
    world.insert_resource(systems);
}
impl<E: SystemInput> Default for RegisteredSystems<E> {
    fn default() -> Self {
        RegisteredSystems { v: SmallVec::new(), tid: TypeIdMap::default() }
    }
}
pub trait Eventy: SystemInput{}

impl World {
    pub fn send<'a,'b,E>(&mut self, event: E)
    where
        E: Event + SystemInput<Inner<'static> = E>,
    {
        run_this_event_system::<E>(event, self);
    }

    pub fn register_event_system<I, Out, F, M>(&mut self, f: F)
    where
        I: SystemInput + 'static,
        Out: Event,
        F: IntoEventSystem<I,Out, M> + 'static,
        M: 'static,
    {
        register_system(self, f);
    }
}
impl<E: Event> SystemInput for &E {
    type Param<'i> = &'i E;
    type Inner<'i> = &'i E;
    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}

