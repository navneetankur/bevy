use core::ops::{Deref, DerefMut};

use crate::{archetype::Archetype, component::Tick, query::{QueryData, With}, system::{Single, SystemMeta, SystemParam}, world::{unsafe_world_cell::UnsafeWorldCell, World}};

pub struct PlayerMarker;
impl crate::component::Component for PlayerMarker {
    const STORAGE_TYPE: crate::component::StorageType = crate::component::StorageType::Table;

    type Mutability = crate::component::Mutable;
}

pub struct Player<'w, Q: QueryData>(Q::Item<'w>);
/// keep in sync with implementation of single.
unsafe impl<'a, D: QueryData + 'static> SystemParam for Player<'a, D> {
    type State = <Single<'a, D, With<PlayerMarker>> as SystemParam>::State;

    type Item<'world, 'state> = Player<'world, D>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        <Single<'a, D, With<PlayerMarker>> as SystemParam>::init_state(world, system_meta)
    }
    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        <Single<'a, D, With<PlayerMarker>> as SystemParam>::new_archetype(state, archetype, system_meta);
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        change_tick: Tick,
    ) -> Self::Item<'world, 'state> {
        let single = <Single<'a, D, With<PlayerMarker>> as SystemParam>::get_param(state, system_meta, world, change_tick);
        let inner = single.into_inner();
        return Player(inner);
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        <Single<'a, D, With<PlayerMarker>> as SystemParam>::apply(state, system_meta, world)
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: crate::world::DeferredWorld) {
        <Single<'a, D, With<PlayerMarker>> as SystemParam>::queue(state, system_meta, world)
    }

    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> Result<(), crate::system::SystemParamValidationError> {
        <Single<'a, D, With<PlayerMarker>> as SystemParam>::validate_param(state, system_meta, world)
    }
}
impl<'w, Q: QueryData> Player<'w, Q> {
    pub fn into_inner(self) -> Q::Item<'w> { self.0 }
}
impl<'w, Q: QueryData> Deref for Player<'w, Q> {
    type Target = Q::Item<'w>;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl<'w, Q: QueryData> DerefMut for Player<'w, Q> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
