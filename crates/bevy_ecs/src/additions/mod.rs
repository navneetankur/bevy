pub mod extras;
use core::ops::Deref;

pub use extras::Extras;
pub mod wcommands;

pub mod player;
use crate::{packet::OptionPacket, system::{ReadOnlySystemParam, Res, ResMut, System, SystemIn, SystemParam}, world::World};
use crate::resource::Resource;

impl World {
    pub fn run_once_with<T, In, Out, Marker>(
        &mut self,
        input: SystemIn<'_, T::System>,
        system: T,
    )
    where
        Out: OptionPacket,
        T: crate::system::IntoSystem<In, Out, Marker>,
        In: crate::packet::SystemInput,
    {
        let mut system: T::System = crate::system::IntoSystem::into_system(system);
        System::initialize(&mut system, self);
        system.run(input, self).expect(system.name().deref()).run(self);
    }
    pub fn run_once<T, Out, Marker>(
        &mut self,
        system: T,
    )
    where
        Out: OptionPacket,
        T: crate::system::IntoSystem<(), Out, Marker>,
    {
        self.run_once_with((), system);
    }
    pub fn run_once_cached_with<T, In, Out, Marker>(
        &mut self,
        input: SystemIn<'_, T::System>,
        system: T,
    )
    where
        Out: OptionPacket,
        T: crate::system::IntoSystem<In, Out, Marker>,
        In: crate::packet::SystemInput,
    {
        self.run_once_np_cached_with(input, system).run(self);
    }
    pub fn run_once_cached<T, Out, Marker>(
        &mut self,
        system: T,
    )
    where
        Out: OptionPacket,
        T: crate::system::IntoSystem<(), Out, Marker>,
    {
        self.run_once_cached_with((), system);
    }
    pub fn run_once_np_cached_with<T, In, Out, Marker>(
        &mut self,
        input: SystemIn<'_, T::System>,
        system: T,
    ) -> Out
    where
        T: crate::system::IntoSystem<In, Out, Marker>,
        In: crate::prelude::SystemInput,
    {
        struct ASystem<S: System>(S);
        impl<S: System> Resource for ASystem<S> {}
        // don't forget to put back.
        if let Some(mut a_system) = self.remove_resource::<ASystem<T::System>>() {
            let rv = a_system.0.run(input, self).expect(a_system.0.name().deref());
            // put back.
            self.insert_resource(a_system);
            return rv;
        }
        let mut system: T::System = crate::system::IntoSystem::into_system(system);
        System::initialize(&mut system, self);
        let rv = system.run(input, self).expect(system.name().deref());
        self.insert_resource(ASystem(system));
        return rv;
    }
}

unsafe impl<'a, T: Resource> SystemParam for &'a T {
    type State = <Res<'a, T> as SystemParam>::State;

    type Item<'world, 'state> = &'world T;

    fn init_state(world: &mut World) -> Self::State {
        <Res<'a, T> as SystemParam>::init_state(world)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut crate::system::SystemMeta,
        component_access_set: &mut crate::query::FilteredAccessSet,
        world: &mut World,
    ) {
        <Res<'a, T> as SystemParam>::init_access(state, system_meta, component_access_set, world);
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        <Res<'a, T> as SystemParam>::get_param(state, system_meta, world, change_tick).into_inner()
    }

    fn apply(state: &mut Self::State, system_meta: &crate::system::SystemMeta, world: &mut World) {
        <Res<'a, T> as SystemParam>::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &crate::system::SystemMeta, world: crate::world::DeferredWorld) {
        <Res<'a, T> as SystemParam>::queue(state, system_meta, world);
    }

    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell,
    ) -> Result<(), crate::system::SystemParamValidationError> {
        <Res<'a, T> as SystemParam>::validate_param(state, system_meta, world)
    }
}
// SAFETY: Res only reads a single World resource
unsafe impl<'a, T: Resource> ReadOnlySystemParam for &'a T {}

unsafe impl<'a, T: Resource> SystemParam for &'a mut T {
    type State = <ResMut<'a, T> as SystemParam>::State;

    type Item<'world, 'state> = &'world mut T;

    fn init_state(world: &mut World) -> Self::State {
        <ResMut<'a, T> as SystemParam>::init_state(world)
    }

    fn init_access(
        state: &Self::State,
        system_meta: &mut crate::system::SystemMeta,
        component_access_set: &mut crate::query::FilteredAccessSet,
        world: &mut World,
    ) {
        <ResMut<'a, T> as SystemParam>::init_access(state, system_meta, component_access_set, world);
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        change_tick: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        <ResMut<'a, T> as SystemParam>::get_param(state, system_meta, world, change_tick).into_inner()
    }

    fn apply(state: &mut Self::State, system_meta: &crate::system::SystemMeta, world: &mut World) {
        <ResMut<'a, T> as SystemParam>::apply(state, system_meta, world);
    }

    fn queue(state: &mut Self::State, system_meta: &crate::system::SystemMeta, world: crate::world::DeferredWorld) {
        <ResMut<'a, T> as SystemParam>::queue(state, system_meta, world);
    }

    unsafe fn validate_param(
        state: &mut Self::State,
        system_meta: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell,
    ) -> Result<(), crate::system::SystemParamValidationError> {
        <ResMut<'a, T> as SystemParam>::validate_param(state, system_meta, world)
    }
}
