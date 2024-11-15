use std::borrow::Cow;

use crate::{
    archetype::{ArchetypeComponentId},
    component::{ComponentId, Tick},
    query::Access,
    system::{
        FunctionSystem, IntoSystem, IsFunctionSystem, System, SystemIn, SystemInput, SystemParamFunction
    },
    world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World},
};

use super::{OptionEvent};

pub trait IntoEventSystem<In: SystemInput, Out, Marker>: Sized {
    type System: System<In = In, Out = ()>;
    fn into_system(this: Self) -> Self::System;
}
pub struct EventSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    inner: FunctionSystem<Marker, F>,
}
impl<Marker, F> IntoEventSystem<F::In, F::Out, (IsFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Out: OptionEvent,
    // <<F as SystemParamFunction<Marker>>::Out as OptionEvent>::Event: SystemInput<Inner<'static> = <<F as SystemParamFunction<Marker>>::Out as OptionEvent>::Event>,
{
    type System = EventSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        let inner = IntoSystem::into_system(func);
        return EventSystem { inner };
    }
}
impl<Marker, F> System for EventSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Out: OptionEvent,
    // <<F as SystemParamFunction<Marker>>::Out as OptionEvent>::Event: SystemInput<Inner<'static> = <<F as SystemParamFunction<Marker>>::Out as OptionEvent>::Event>,
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
        _: SystemIn<'_, Self>,
        _: UnsafeWorldCell,
    ) -> Self::Out {
        unimplemented!("no parallelism use run");
    }
    /// Runs the system with the given input in the world.
    ///
    /// For [read-only](ReadOnlySystem) systems, see [`run_readonly`], which can be called using `&World`.
    ///
    /// Unlike [`System::run_unsafe`], this will apply deferred parameters *immediately*.
    ///
    /// [`run_readonly`]: ReadOnlySystem::run_readonly
    fn run(&mut self, input: SystemIn<'_, Self>, world: &mut World) -> Self::Out {
        let out = self.inner.run(input, world);
        out.run(world);
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        self.inner.apply_deferred(world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
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
