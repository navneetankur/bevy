pub mod extras;
pub use extras::Extras;
pub mod wcommands;

pub mod player;
use crate::{packet::OptionPacket, system::{ReadOnlySystemParam, Res, ResMut, Resource, System, SystemIn, SystemParam}, world::World};

impl World {
    pub fn run_once_with<T, In, Out, Marker>(
        mut self,
        input: SystemIn<'_, T::System>,
        system: T,
    )
    where
        Out: OptionPacket,
        T: crate::system::IntoSystem<In, Out, Marker>,
        In: crate::packet::SystemInput,
    {
        let mut system: T::System = crate::system::IntoSystem::into_system(system);
        System::initialize(&mut system, &mut self);
        system.run(input, &mut self).run(&mut self);
    }
    pub fn run_once<T, Out, Marker>(
        self,
        system: T,
    )
    where
        Out: OptionPacket,
        T: crate::system::IntoSystem<(), Out, Marker>,
    {
        self.run_once_with((), system);
    }
    pub fn run_once_cached_with<T, In, Out, Marker>(
        mut self,
        input: SystemIn<'_, T::System>,
        system: T,
    )
    where
        Out: OptionPacket,
        T: crate::system::IntoSystem<In, Out, Marker>,
        In: crate::packet::SystemInput,
    {
        struct ASystem<S: System>(S);
        impl<S: System> Resource for ASystem<S> {}
        // don't forget to put back.
        if let Some(mut a_system) = self.remove_resource::<ASystem<T::System>>() {
            a_system.0.run(input, &mut self).run(&mut self);
            // put back.
            self.insert_resource(a_system);
            return;
        }
        let mut system: T::System = crate::system::IntoSystem::into_system(system);
        System::initialize(&mut system, &mut self);
        system.run(input, &mut self).run(&mut self);
        self.insert_resource(ASystem(system));
    }
    pub fn run_once_cached<T, Out, Marker>(
        self,
        system: T,
    )
    where
        Out: OptionPacket,
        T: crate::system::IntoSystem<(), Out, Marker>,
    {
        self.run_once_cached_with((), system);
    }
}

unsafe impl<'a, T: Resource> SystemParam for &'a T {
    type State = <Res<'a, T> as SystemParam>::State;

    type Item<'w, 's> = &'w T;

    fn init_state(world: &mut World, system_meta: &mut crate::system::SystemMeta) -> Self::State {
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

    fn init_state(world: &mut World, system_meta: &mut crate::system::SystemMeta) -> Self::State {
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
