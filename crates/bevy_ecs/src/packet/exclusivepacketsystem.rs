use std::borrow::Cow;

use crate::{archetype::ArchetypeComponentId, component::{ComponentId, Tick}, query::Access, system::{ExclusiveFunctionSystem, ExclusiveSystemParamFunction, IntoSystem, IsExclusiveFunctionSystem, System, SystemIn}, world::{unsafe_world_cell::UnsafeWorldCell, World}};

use super::{packetsystem::IntoPacketSystem, OptionPacket};

pub struct ExclusivePacketSystem<Marker, F>
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    inner: ExclusiveFunctionSystem<Marker, F>,
}
impl<Marker, F> IntoPacketSystem<F::In, F::Out, (IsExclusiveFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
    F::Out: OptionPacket,
{
    type System = ExclusivePacketSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        ExclusivePacketSystem {
            inner: IntoSystem::into_system(func)
        }
    }
}

impl<Marker, F> System for ExclusivePacketSystem<Marker, F>
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
    F::Out: OptionPacket,
{
    type In = F::In;
    type Out = ();

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        self.inner.name()
    }

    #[inline]
    fn component_access(&self) -> &Access<ComponentId> {
        self.inner.component_access()
    }

    #[inline]
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        self.inner.archetype_component_access()
    }

    #[inline]
    fn is_send(&self) -> bool {
        self.inner.is_send()
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        self.inner.is_exclusive()
    }

    #[inline]
    fn has_deferred(&self) -> bool {
        self.inner.has_deferred()
    }

    #[inline]
    unsafe fn run_unsafe(
        &mut self,
        _input: SystemIn<'_, Self>,
        _world: UnsafeWorldCell,
    ) -> Self::Out {
        panic!("Cannot run exclusive systems with a shared World reference");
    }

    fn run(&mut self, input: SystemIn<'_, Self>, world: &mut World) -> Self::Out {
        let out = <ExclusiveFunctionSystem<Marker, F> as System>::run(&mut self.inner, input, world);
        out.run(world);
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.inner.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: crate::world::DeferredWorld) {
        self.inner.queue_deferred(world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.inner.initialize(world);
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        self.inner.update_archetype_component_access(world);
    }

    #[inline]
    fn check_change_tick(&mut self, change_tick: Tick) {
        self.inner.check_change_tick(change_tick);
    }
    fn get_last_run(&self) -> Tick {
        self.inner.get_last_run()
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.inner.set_last_run(last_run);
    }

    unsafe fn validate_param_unsafe(
        &mut self,
        world: UnsafeWorldCell,
    ) -> Result<(), crate::system::SystemParamValidationError> {
        self.inner.validate_param_unsafe(world)
    }
}
