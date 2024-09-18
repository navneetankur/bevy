use core::marker::PhantomData;
use std::borrow::Cow;

use crate::{archetype::ArchetypeComponentId, component::{ComponentId, Tick}, event::Event, query::Access, system::{boxysystem::IntoBoxySystem, check_system_change_tick, ExclusiveSystemParam, System, SystemMeta}, world::{unsafe_world_cell::UnsafeWorldCell, World}};

use super::{ExclusiveSystemParamFunction, IsExclusiveFunctionSystem, PARAM_MESSAGE};

pub struct BoxyExclusiveFunctionSystem<Marker, F>
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    func: F,
    param_state: Option<<F::Param as ExclusiveSystemParam>::State>,
    system_meta: SystemMeta,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> Marker>,
}
impl<Marker, F> IntoBoxySystem<F::In, F::Out, (IsExclusiveFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
    F::Out : Event + Clone,
{
    type System = BoxyExclusiveFunctionSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        BoxyExclusiveFunctionSystem {
            func,
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            marker: PhantomData,
        }
    }
}
impl<Marker, F> System for BoxyExclusiveFunctionSystem<Marker, F>
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
    F::Out: Event + Clone,
{
    type In = F::In;
    type Out = Box<dyn Event>;

    #[inline]
    fn name(&self) -> Cow<'static, str> {
        self.system_meta.name.clone()
    }

    #[inline]
    fn component_access(&self) -> &Access<ComponentId> {
        self.system_meta.component_access_set.combined_access()
    }

    #[inline]
    fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> {
        &self.system_meta.archetype_component_access
    }

    #[inline]
    fn is_send(&self) -> bool {
        // exclusive systems should have access to non-send resources
        // the executor runs exclusive systems on the main thread, so this
        // field reflects that constraint
        false
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        true
    }

    #[inline]
    fn has_deferred(&self) -> bool {
        // exclusive systems have no deferred system params
        false
    }

    #[inline]
    unsafe fn run_unsafe(&mut self, _input: Self::In, _world: UnsafeWorldCell) -> Self::Out {
        panic!("Cannot run exclusive systems with a shared World reference");
    }

    fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out {
        world.last_change_tick_scope(self.system_meta.last_run, |world| {
            #[cfg(feature = "trace")]
            let _span_guard = self.system_meta.system_span.enter();

            let params = F::Param::get_param(
                self.param_state.as_mut().expect(PARAM_MESSAGE),
                &self.system_meta,
            );
            let out = self.func.run(world, input, params);

            world.flush();
            self.system_meta.last_run = world.increment_change_tick();

            Box::new(out)
        })
    }

    #[inline]
    fn apply_deferred(&mut self, _world: &mut World) {
        // "pure" exclusive systems do not have any buffers to apply.
        // Systems made by piping a normal system with an exclusive system
        // might have buffers to apply, but this is handled by `PipeSystem`.
    }

    #[inline]
    fn queue_deferred(&mut self, _world: crate::world::DeferredWorld) {
        // "pure" exclusive systems do not have any buffers to apply.
        // Systems made by piping a normal system with an exclusive system
        // might have buffers to apply, but this is handled by `PipeSystem`.
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        self.system_meta.last_run = world.change_tick().relative_to(Tick::MAX);
        self.param_state = Some(F::Param::init(world, &mut self.system_meta));
    }

    fn update_archetype_component_access(&mut self, _world: UnsafeWorldCell) {}

    #[inline]
    fn check_change_tick(&mut self, change_tick: Tick) {
        check_system_change_tick(
            &mut self.system_meta.last_run,
            change_tick,
            self.system_meta.name.as_ref(),
        );
    }

    fn get_last_run(&self) -> Tick {
        self.system_meta.last_run
    }

    fn set_last_run(&mut self, last_run: Tick) {
        self.system_meta.last_run = last_run;
    }
}
