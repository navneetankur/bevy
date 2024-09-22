pub mod eventsystem;
use core::any::TypeId;
use crate::{self as bevy_ecs, system::SystemInput};
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
    fn run_systems(&self, world: &mut World);
}
impl Event for (){ fn run_systems(&self, _: &mut World) {} }
pub fn register_system<I, Out, F, M>(world: &mut World, f: F)
where
    I: SystemInput + 'static,
    Out: Event,
    // F: SystemParamFunction<M, In = I, Out = Out>,
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
pub fn run_this_event_system<'a, I>(event: I::Inner<'a>, world: &mut World)
where 
    I: SystemInput + 'static,
    I::Inner<'a>: Copy,
{
    //don't forget to put it back.
    let Some(mut systems) = world.remove_resource::<RegisteredSystems<I>>() else {return};
    for system in &mut systems.v {
        let new_event = system.v.run(event, world);
        new_event.run_systems(world);
    }
    debug_assert!(!world.contains_resource::<RegisteredSystems<I>>());
    world.insert_resource(systems);


    // world.init_resource::<RegisteredSystems<I>>();
    // world.resource_scope(
    //     |world: &mut World, mut systems: Mut<RegisteredSystems<I>>| {
    //         for system in &mut systems.v {
    //             let new_event = system.v.run(event, world);
    //             new_event.run_systems(world);
    //         }
    //     },
    // );
}
impl<E: SystemInput> Default for RegisteredSystems<E> {
    fn default() -> Self {
        RegisteredSystems { v: SmallVec::new(), tid: TypeIdMap::default() }
    }
}

impl World {
    pub fn send_ref<'a,'b,E>(&mut self, event: <&'b E as SystemInput>::Inner<'a>)
    where
        E: Event,
        &'b E: SystemInput + 'static,
        <&'b E as SystemInput>::Inner<'a>: Copy,
    {
        run_this_event_system::<&E>(event, self);
    }
    pub fn send<'a,'b,E>(&mut self, event: E)
    where
        E: Event,
    {
        self.send_ref(&event);
    }

    pub fn register_event_system<I, Out, F, M>(&mut self, f: F)
    where
        I: SystemInput + Copy + 'static,
        Out: Event,
        // F: SystemParamFunction<M, In = I, Out = Out>,
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

