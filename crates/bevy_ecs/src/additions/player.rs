use core::ops::{Deref, DerefMut};

use crate::{component::Tick, query::{QueryData, With}, system::{Single, SystemMeta, SystemParam}, world::{unsafe_world_cell::UnsafeWorldCell, World}};

pub struct PlayerMarker;
impl crate::component::Component for PlayerMarker {
    const STORAGE_TYPE: crate::component::StorageType = crate::component::StorageType::Table;

    type Mutability = crate::component::Mutable;
}

pub struct Player<'w, 's, Q: QueryData>(Q::Item<'w, 's>);
/// keep in sync with implementation of single.
unsafe impl<'w, 's, D: QueryData + 'static> SystemParam for Player<'w, 's, D> {
    type State = <Single<'w, 's, D, With<PlayerMarker>> as SystemParam>::State;

    type Item<'world, 'state> = Player<'world, 'state, D>;

    fn init_state(world: &mut World) -> Self::State {
        <Single<'w, 's, D, With<PlayerMarker>> as SystemParam>::init_state(world)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut SystemMeta,
        component_access_set: &mut crate::query::FilteredAccessSet,
        world: &mut World,
    ) {
        <Single<'w, 's, D, With<PlayerMarker>> as SystemParam>::init_access(state, system_meta, component_access_set, world);
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        Player(
            <Single<'w, 's, D, With<PlayerMarker>> as SystemParam>::get_param(state, system_meta, world, change_tick)
                .into_inner()
        )
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        <Single<'w, 's, D, With<PlayerMarker>> as SystemParam>::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: crate::world::DeferredWorld) {
        <Single<'w, 's, D, With<PlayerMarker>> as SystemParam>::queue(state, system_meta, world);
    }

    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), crate::system::SystemParamValidationError> {
        <Single<'w, 's, D, With<PlayerMarker>> as SystemParam>::validate_param(state, system_meta, world)
    }
}
impl<'w, 's, Q: QueryData> Player<'w, 's, Q> {
    pub fn into_inner(self) -> Q::Item<'w, 's> { self.0 }
}
impl<'w, 's, Q: QueryData> Deref for Player<'w, 's, Q> {
    type Target = Q::Item<'w, 's>;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl<'w, 's, Q: QueryData> DerefMut for Player<'w, 's, Q> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
