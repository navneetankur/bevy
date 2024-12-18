use core::ops::{Deref, DerefMut};

use bevy_utils::synccell::SyncCell;

use crate::{event::RegisteredSystems, system::{ReadOnlySystemParam, SystemParam}, world::World};

use super::{run_this_event_system, Event, SmolId, SystemInput};

pub struct EventSlicer<'s, E: Event, const FORWARD: bool = true>(&'s mut Vec<E>);
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
    for<'b> &'b E: SmolId,
    for<'c> &'c [E]: SmolId,
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
               // todo optimization: get the systems and run one it
               // instead of getting event multiple times and hitting hashmap in world.
               run_this_event_system::<false, E>(event, world);
           }
        }
        else {
            state.get().clear();
        }
    }
}
unsafe impl<'a, E: Event, const FORWARD: bool> ReadOnlySystemParam for EventSlicer<'a, E, FORWARD>
where 
    E: SystemInput<Inner<'static> = E>,
    for<'b> &'b E: SmolId,
    for<'c> &'c [E]: SmolId,
{}

impl<E: Event> SystemInput for &[E] {
    type Param<'i> = &'i [E];
    type Inner<'i> = &'i [E];
    fn wrap(this: Self::Inner<'_>) -> Self::Param<'_> { this }
}
fn run_for_slice_event<E>(world: &mut World, event_slice: &[E])
where
    E: Event,
    for<'b> &'b E: SmolId,
    for<'c> &'c [E]: SmolId,
{
    //don't forget to put it back.
    let Some(mut systems) = world.remove_event_system::<&[E]>() else {return;};
    for system in &mut systems.v {
        system.v.run(event_slice, world);
    }
    //put back
    world.put_back_event_system(systems);
}

