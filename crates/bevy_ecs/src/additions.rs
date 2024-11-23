pub mod player;
use crate::system::{ReadOnlySystemParam, Res, ResMut, Resource, SystemParam};

unsafe impl<'a, T: Resource> SystemParam for &'a T {
    type State = <Res<'a, T> as SystemParam>::State;

    type Item<'w, 's> = &'w T;

    fn init_state(world: &mut crate::prelude::World, system_meta: &mut crate::system::SystemMeta) -> Self::State {
        <Res<'a, T> as SystemParam>::init_state(world, system_meta)
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        let p = <Res<'a, T> as SystemParam>::get_param(state, system_meta, world, change_tick);
        return p.value;
    }
}
// SAFETY: Res only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for &'a T {}

unsafe impl<'a, T: Resource> SystemParam for &'a mut T {
    type State = <ResMut<'a, T> as SystemParam>::State;

    type Item<'w, 's> = &'w mut T;

    fn init_state(world: &mut crate::prelude::World, system_meta: &mut crate::system::SystemMeta) -> Self::State {
        <ResMut<'a, T> as SystemParam>::init_state(world, system_meta)
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        let mut p = <ResMut<'a, T> as SystemParam>::get_param(state, system_meta, world, change_tick);
        let _ = p.as_mut();
        return p.value;
    }
}
