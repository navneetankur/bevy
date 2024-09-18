use core::any::TypeId;

use crate::{system::{IntoSystem, System}, world::Mut};
use bevy_utils::TypeIdMap;
use smallvec::SmallVec;

use crate::{system::BoxedSystem, world::World};

pub struct EventInSystem<E: Event + Clone> {
    pub v: BoxedSystem<E, Box<dyn Event>>,
    pub tid: TypeId,
}
pub struct RegisteredSystems<E: Event + Clone>{
    pub v: SmallVec<[EventInSystem<E>; 1]>,
    pub tid: TypeIdMap<usize>,
}

pub trait Event: Send + Sync + 'static {
    fn run_systems(&self, world: &mut World);
}
pub fn register_system<E, Out, F, M>(world: &mut World, f: F)
where
    E: Event + Clone,
    Out: Event + Clone,
    F: IntoSystem<E, Out, M> + 'static,
{
    world.init_resource::<RegisteredSystems<E>>();
    world.resource_scope(|world: &mut World, mut systems: Mut<RegisteredSystems<E>>| {
        let tid = TypeId::of::<F>();
        if systems.tid.contains_key(&tid) { return };

        let system = IntoSystem::into_system(f);
    });
}

impl<E> Event for E
where E: Send + Sync + Clone + 'static
{
    fn run_systems(&self, world: &mut World) {
        run_this_event_system(self, world);
    }
}
pub fn run_this_event_system<E: Event + Clone>(event: &E, world: &mut World) {
    world.init_resource::<RegisteredSystems<E>>();
    world.resource_scope(
        |world: &mut World, mut systems: Mut<RegisteredSystems<E>>| {
            for system in &mut systems.v {
                let new_event = system.v.run(event.clone(), world);
                new_event.run_systems(world);
            }
        },
    );
}
impl<E: Event + Clone> Default for RegisteredSystems<E> {
    fn default() -> Self {
        RegisteredSystems { v: SmallVec::new(), tid: TypeIdMap::default() }
    }
}
