use core::ops::{Deref, DerefMut};

use bevy_utils::synccell::SyncCell;

use crate::{event::RegisteredSystems, system::SystemParam, world::World};

use super::{run_this_event_system, Event, SystemInput};

pub struct EventSlicer<'s, E: Event, const FORWARD: bool = false>(&'s mut Vec<E>);
impl<'s, E: Event, const F: bool> EventSlicer<'s, E, F> {
    fn new(v: &'s mut Vec<E>) -> Self { Self(v) }
}
impl<'s, E: Event, const FORWARD: bool> Deref for EventSlicer<'s, E, FORWARD> {
    type Target = Vec<E>;
    fn deref(&self) -> &Self::Target { self.0 }
}
impl<'s, E: Event, const FORWARD: bool> DerefMut for EventSlicer<'s, E, FORWARD> {
    fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}



unsafe impl<'a, E: Event, const FORWARD: bool> SystemParam for EventSlicer<'a, E, FORWARD>
where 
    E: SystemInput<Inner<'static> = E>,
{
    type State = SyncCell<Vec<E>>;

    type Item<'world, 'state> = EventSlicer<'state, E, FORWARD>;

    fn init_state(_: &mut World, _: &mut crate::system::SystemMeta) -> Self::State {
        SyncCell::new(Vec::with_capacity(2))
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        _: &crate::system::SystemMeta,
        _: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        _: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        EventSlicer::new(state.get())
    }

    fn apply(state: &mut Self::State, _: &crate::system::SystemMeta, world: &mut World) {
        run_for_slice_event(world, state.get());
        if FORWARD {
           for event in state.get().drain(..) {
               run_this_event_system(event, world);
           }
        }
        else {
            state.get().clear();
        }
    }
}

impl<E: Event> SystemInput for &[E] {
    type Param<'i> = &'i [E];
    type Inner<'i> = &'i [E];
    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> { this }
}
fn run_for_slice_event<E>(world: &mut World, event_slice: &[E]) where E: Event {
    //don't forget to put it back.
    let Some(mut systems) = world.remove_resource::<RegisteredSystems<&[E]>>() else {return};
    for system in &mut systems.v {
        system.v.run(event_slice, world);
    }
    debug_assert!(!world.contains_resource::<RegisteredSystems<&[E]>>());
    world.insert_resource(systems);
}

