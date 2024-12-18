pub mod eventsystem;
pub mod exclusiveeventsystem;
pub mod eventslicer;
mod optionevent;
pub use optionevent::OptionEvent;
use core::{any::{type_name, TypeId}, sync::atomic::AtomicU32};
use crate::{self as bevy_ecs};
pub use crate::system::SystemInput;
pub use bevy_ecs_macros::Event;
use eventsystem::IntoEventSystem;
use crate::system::{Resource, System};
use bevy_utils::TypeIdMap;
use smallvec::SmallVec;
use crate::{system::BoxedSystem, world::World};

pub static NEXT_EVENT_ID: AtomicU32 = AtomicU32::new(0);

pub struct EventInSystem<E: SystemInput> {
    pub v: BoxedSystem<E, ()>,
    pub tid: TypeId,
}
pub struct RegisteredSystems<E: SystemInput>{
    pub v: SmallVec<[EventInSystem<E>; 1]>,
    pub tid: TypeIdMap<usize>,
}
// pub trait IRegisteredSystem {
//     fn as_registered_systems<E: SystemInput>(&mut self) -> &mut RegisteredSystems<E>;
// }
// impl<E: SystemInput> IRegisteredSystem for RegisteredSystems<E>{
//     fn as_registered_systems<F: SystemInput>(&mut self) -> &mut RegisteredSystems<F> {self}
// }

pub trait Event: Send + Sync + SystemInput + SmolId + 'static { }
pub trait SmolId { fn sid() -> usize; }

pub fn register_system<I, Out, F, M>(world: &mut World, f: F)
where
    I: SystemInput + SmolId + 'static,
    Out: OptionEvent,
    F: IntoEventSystem<I, Out, M> + 'static,
    M: 'static,
{
    #[cfg(debug_assertions)]
    world.init_resource::<EventInMotion>();
    // don't forget to put it back.
    let mut systems = world.remove_event_system::<I>().unwrap_or_default();

    let tid = TypeId::of::<F>();
    if systems.tid.contains_key(&tid) { return };
    let mut system = IntoEventSystem::into_system(f);
    system.initialize(world);
    let system = EventInSystem { v: Box::new(system), tid };
    systems.v.push(system);
    let index = systems.v.len() - 1;
    systems.tid.insert(tid, index);

    // put back here.
    world.put_back_event_system(systems);
}
#[derive(Resource, Default)]
struct EventInMotion(Vec<TypeId>);
pub fn run_this_event_system<'a, const SLICE: bool, E>(event: E, world: &mut World)
where 
    E: Event,
    E: SystemInput<Inner<'static> = E>,
    for<'b> &'b E: SmolId,
    for<'c> &'c [E]: SmolId,
{
    #[cfg(debug_assertions)]
    {
        let mut in_motion = world.resource_mut::<EventInMotion>();
        if in_motion.0.contains(&TypeId::of::<E>()) {
            panic!("Recursive event {:?}", type_name::<E>());
        } else {
            in_motion.0.push(TypeId::of::<E>());
        }
    }
    run_for_ref_event(world, &event);
    if SLICE { run_for_slice_event(world, &event); }
    run_for_val_event(world, event);
    #[cfg(debug_assertions)]
    {
        let mut in_motion = world.resource_mut::<EventInMotion>();
        let index = in_motion.0.iter().position(|x| *x == TypeId::of::<E>()).unwrap();
        in_motion.0.swap_remove(index);
    }
}

fn run_for_val_event<E>(world: &mut World, event: E)
where
    E: Event,
    E: SystemInput<Inner<'static> = E>
{
    world.with_event_system::<E>(|world, systems| {
        let mut systems_iter = systems.v.iter_mut();
        let Some(system) = systems_iter.next() else { return };
        system.v.run(event, world);
        debug_assert!(systems_iter.next().is_none(), "Only one system can take value {:?}", type_name::<E>());
    });
}

fn run_for_ref_event<E>(world: &mut World, event: &E)
where
    E: Event,
    for<'a> &'a E: SmolId,
    // E: SystemInput<Inner<'static> = E>
{
    world.with_event_system::<&E>(|world, systems| {
        for system in &mut systems.v {
            system.v.run(event, world);
        }
    });
}
fn run_for_slice_event<E>(world: &mut World, event: &E)
where
    E: Event,
    for<'a> &'a [E]: SmolId,
    // E: SystemInput<Inner<'static> = E>
{
    //don't forget to put it back.
    // let Some(mut systems) = world.remove_event_system::<&[E]>() else {return;};
    world.with_event_system::<&[E]>(|world, systems| {
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
        RegisteredSystems { v: SmallVec::new(), tid: TypeIdMap::default() }
    }
}
pub trait Eventy: SystemInput{}

impl World {
    pub fn send<'a,'b,E>(&mut self, event: E)
    where
        E: Event + SystemInput<Inner<'static> = E>,
        for<'d> &'d E: SmolId,
        for<'c> &'c [E]: SmolId,
    {
        run_this_event_system::<true, E>(event, self);
    }

    pub fn register_event_system<I, Out, F, M>(&mut self, f: F)
    where
        I: SystemInput + SmolId+ 'static,
        Out: OptionEvent,
        F: IntoEventSystem<I,Out, M> + 'static,
        M: 'static,
    {
        register_system(self, f);
    }

    fn with_event_system<I>(&mut self, f: impl FnOnce(&mut World, &mut RegisteredSystems<I>),)
    where 
        I: SystemInput + SmolId + 'static,
    {
        let Some(mut systems) = self.remove_event_system::<I>() else {return};
        f(self, &mut systems);
        self.put_back_event_system(systems);
    }

    /// don't forget to put it back.
    fn remove_event_system<I: SystemInput + SmolId + 'static>(&mut self) -> Option<Box<RegisteredSystems<I>>> {
        let event_systems = &mut self.extras.event_systems;
        let event_index = I::sid();
        if event_systems.len() <= event_index {
            event_systems.resize_with(event_index + 1, || None);
            return None;
        }
        let rv = event_systems[event_index].take();
        return rv.map(|v| v.downcast().unwrap());
    }

    fn put_back_event_system<I: SystemInput + SmolId + 'static>(&mut self, systems: Box<RegisteredSystems<I>>) {
        let event_systems = &mut self.extras.event_systems;
        let event_index = I::sid();
        debug_assert!(event_systems[event_index].is_none());
        event_systems[event_index] = Some(systems);
    }

}
impl<E: Event> SystemInput for &E {
    type Param<'i> = &'i E;
    type Inner<'i> = &'i E;
    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> {
        this
    }
}
impl<E: Event + SmolId> SmolId for &E {
    fn sid() -> usize { E::sid() + 1 }
}
impl<E: Event + SmolId> SmolId for &[E] {
    fn sid() -> usize { E::sid() + 2 }
}
