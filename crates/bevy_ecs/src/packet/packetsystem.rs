use core::ops::Deref;

use crate::{
    component::{Tick},
    system::{
        FunctionSystem, IntoSystem, IsFunctionSystem, System, SystemIn, SystemInput, SystemParamFunction
    },
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

use super::OptionPacket;

pub trait IntoPacketSystem<In: SystemInput, Out, Marker>: Sized {
    type System: System<In = In, Out = ()>;
    fn into_system(this: Self) -> Self::System;
}
pub struct FunctionPacketSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    inner: FunctionSystem<Marker, F::Out, F>,
}
impl<Marker, F> IntoPacketSystem<F::In, F::Out, (IsFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Out: OptionPacket,
    // <<F as SystemParamFunction<Marker>>::Out as OptionEvent>::Event: SystemInput<Inner<'static> = <<F as SystemParamFunction<Marker>>::Out as OptionEvent>::Event>,
{
    type System = FunctionPacketSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        let inner = IntoSystem::into_system(func);
        return FunctionPacketSystem { inner };
    }
}
impl<Marker, F> System for FunctionPacketSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Out: OptionPacket,
    // <<F as SystemParamFunction<Marker>>::Out as OptionEvent>::Event: SystemInput<Inner<'static> = <<F as SystemParamFunction<Marker>>::Out as OptionEvent>::Event>,
{
    type In = F::In;

    type Out = ();

    fn name(&self) -> bevy_utils::prelude::DebugName {
        self.inner.name()
    }

    fn flags(&self) -> crate::system::SystemStateFlags {
        self.inner.flags()
    }

    unsafe fn run_unsafe(
        &mut self,
        _: SystemIn<'_, Self>,
        _: UnsafeWorldCell,
    ) -> Result<Self::Out, crate::system::RunSystemError> {
        unimplemented!("no parallelism use run");
    }

    fn apply_deferred(&mut self, world: &mut World) {
        self.inner.apply_deferred(world);
    }

    fn queue_deferred(&mut self, world: DeferredWorld) {
        self.inner.queue_deferred(world);
    }

    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), crate::system::SystemParamValidationError> {
        self.inner.validate_param_unsafe(world)
    }

    fn initialize(&mut self, world: &mut World) -> crate::query::FilteredAccessSet {
        self.inner.initialize(world)
    }

    fn check_change_tick(&mut self, check: crate::component::CheckChangeTicks) {
        self.inner.check_change_tick(check);
    }

    fn get_last_run(&self) -> Tick {
        self.inner.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.inner.set_last_run(last_run);
    }

    fn type_id(&self) -> core::any::TypeId {
        self.inner.type_id()
    }

    fn is_send(&self) -> bool {
        self.inner.is_send()
    }

    fn is_exclusive(&self) -> bool {
        self.inner.is_exclusive()
    }

    fn has_deferred(&self) -> bool {
        self.inner.has_deferred()
    }

    fn run(
        &mut self,
        input: SystemIn<'_, Self>,
        world: &mut World,
    ) -> Result<Self::Out, crate::system::RunSystemError> {
        let out = self.inner.run(input, world).expect(self.inner.name().deref());
        out.run(world);
        return Ok(());
    }

    fn run_without_applying_deferred(
        &mut self,
        _: SystemIn<'_, Self>,
        _: &mut World,
    ) -> Result<Self::Out, crate::system::RunSystemError> {
        unimplemented!()
    }

    fn validate_param(&mut self, world: &World) -> Result<(), crate::system::SystemParamValidationError> {
        self.inner.validate_param(world)
    }

    fn default_system_sets(&self) -> alloc::vec::Vec<crate::schedule::InternedSystemSet> {
        self.inner.default_system_sets()
    }
}
