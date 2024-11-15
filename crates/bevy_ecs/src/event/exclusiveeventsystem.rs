use std::borrow::Cow;

use crate::{archetype::ArchetypeComponentId, component::{ComponentId, Tick}, query::Access, system::{ExclusiveFunctionSystem, ExclusiveSystemParamFunction, IntoSystem, IsExclusiveFunctionSystem, System, SystemIn}, world::{unsafe_world_cell::UnsafeWorldCell, World}};

use super::{eventsystem::IntoEventSystem, OptionEvent};

pub struct ExclusiveEventSystem<Marker, F>
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    inner: ExclusiveFunctionSystem<Marker, F>,
}
impl<Marker, F> IntoEventSystem<F::In, F::Out, (IsExclusiveFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
    F::Out: OptionEvent,
{
    type System = ExclusiveEventSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        ExclusiveEventSystem {
            inner: IntoSystem::into_system(func)
        }
    }
}

impl<Marker, F> System for ExclusiveEventSystem<Marker, F>
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
    F::Out: OptionEvent,
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
        let out = self.inner.run(input, world);
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
}
