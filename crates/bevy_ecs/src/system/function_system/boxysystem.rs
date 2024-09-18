use core::{any::TypeId, marker::PhantomData};
use std::borrow::Cow;

use crate::{archetype::{ArchetypeComponentId, ArchetypeGeneration}, component::{ComponentId, Tick}, event::Event, query::Access, system::{check_system_change_tick, ReadOnlySystem, ReadOnlySystemParam, System, SystemParam}, world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld, World, WorldId}};

use super::{IsFunctionSystem, SystemMeta, SystemParamFunction};

pub trait IntoBoxySystem<In, Out: Event, Marker>: Sized {
    /// The type of [`System`] that this instance converts into.
    type System: System<In = In, Out = Box<dyn Event>>;

    /// Turns this value into its corresponding [`System`].
    fn into_system(this: Self) -> Self::System;

    /// Get the [`TypeId`] of the [`System`] produced after calling [`into_system`](`IntoSystem::into_system`).
    #[inline]
    fn system_type_id(&self) -> TypeId {
        TypeId::of::<Self::System>()
    }
}
pub struct BoxyFunctionSystem<Marker, F>
where
    F: SystemParamFunction<Marker>,
{
    func: F,
    world_id: Option<WorldId>,
    pub(crate) param_state: Option<<F::Param as SystemParam>::State>,
    pub(crate) system_meta: SystemMeta,
    archetype_generation: ArchetypeGeneration,
    // NOTE: PhantomData<fn()-> T> gives this safe Send/Sync impls
    marker: PhantomData<fn() -> Marker>,
}
impl<Marker, F> System for BoxyFunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
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
        self.system_meta.is_send
    }

    #[inline]
    fn is_exclusive(&self) -> bool {
        false
    }

    #[inline]
    fn has_deferred(&self) -> bool {
        self.system_meta.has_deferred
    }

    #[inline]
    unsafe fn run_unsafe(&mut self, input: Self::In, world: UnsafeWorldCell) -> Self::Out  {
        #[cfg(feature = "trace")]
        let _span_guard = self.system_meta.system_span.enter();

        let change_tick = world.increment_change_tick();

        // SAFETY:
        // - The caller has invoked `update_archetype_component_access`, which will panic
        //   if the world does not match.
        // - All world accesses used by `F::Param` have been registered, so the caller
        //   will ensure that there are no data access conflicts.
        let params = unsafe {
            F::Param::get_param(
                self.param_state.as_mut().expect("probably not initialized"),
                &self.system_meta,
                world,
                change_tick,
            )
        };
        let out = self.func.run(input, params);
        self.system_meta.last_run = change_tick;
        Box::new(out)
    }

    #[inline]
    fn apply_deferred(&mut self, world: &mut World) {
        let param_state = self.param_state.as_mut().expect("probably not initialized");
        F::Param::apply(param_state, &self.system_meta, world);
    }

    #[inline]
    fn queue_deferred(&mut self, world: DeferredWorld) {
        let param_state = self.param_state.as_mut().expect("probably not initialized");
        F::Param::queue(param_state, &self.system_meta, world);
    }

    #[inline]
    fn initialize(&mut self, world: &mut World) {
        if let Some(id) = self.world_id {
            assert_eq!(
                id,
                world.id(),
                "System built with a different world than the one it was added to.",
            );
        } else {
            self.world_id = Some(world.id());
            self.param_state = Some(F::Param::init_state(world, &mut self.system_meta));
        }
        self.system_meta.last_run = world.change_tick().relative_to(Tick::MAX);
    }

    fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
        assert_eq!(self.world_id, Some(world.id()), "Encountered a mismatched World. A System cannot be used with Worlds other than the one it was initialized with.");
        let archetypes = world.archetypes();
        let old_generation =
            std::mem::replace(&mut self.archetype_generation, archetypes.generation());

        for archetype in &archetypes[old_generation..] {
            let param_state = self.param_state.as_mut().unwrap();
            // SAFETY: The assertion above ensures that the param_state was initialized from `world`.
            unsafe { F::Param::new_archetype(param_state, archetype, &mut self.system_meta) };
        }
    }

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
unsafe impl<Marker, F> ReadOnlySystem for BoxyFunctionSystem<Marker, F>
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Param: ReadOnlySystemParam,
    F::Out: Event + Clone,
{}
impl<Marker, F> IntoBoxySystem<F::In, F::Out, (IsFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: SystemParamFunction<Marker>,
    F::Out: Clone + Event,
{
    type System = BoxyFunctionSystem<Marker, F>;
    fn into_system(func: Self) -> Self::System {
        BoxyFunctionSystem {
            func,
            param_state: None,
            system_meta: SystemMeta::new::<F>(),
            world_id: None,
            archetype_generation: ArchetypeGeneration::initial(),
            marker: PhantomData,
        }
    }
}
