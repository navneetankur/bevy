use crate::{archetype::Archetype, component::Tick, query::{QueryData, QueryState, ReadOnlyQueryData, With}, system::{init_query_param, ReadOnlySystemParam, SystemMeta, SystemParam}, world::{unsafe_world_cell::UnsafeWorldCell, World}};

pub struct PlayerMarker;
impl crate::component::Component for PlayerMarker {
    const STORAGE_TYPE: crate::component::StorageType = crate::component::StorageType::Table;
}

pub struct Player<'w, Q: QueryData>(pub Q::Item<'w>);
unsafe impl<'w, 's, D: ReadOnlyQueryData + 'static> ReadOnlySystemParam
    for Player<'w, D>
{ }
// onupdate:
// ensure sync with Query as SystemParam.
unsafe impl<D: QueryData + 'static> SystemParam for Player<'_, D> {
    // type State = PlayerQueryState<D>;
    type State = QueryState<D, With<PlayerMarker>>;
    type Item<'w, 's> = Player<'w, D>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        let state = QueryState::new_with_access(world, &mut system_meta.archetype_component_access);
        init_query_param(world, system_meta, &state);
        state
    }

    unsafe fn new_archetype(
        state: &mut Self::State,
        archetype: &Archetype,
        system_meta: &mut SystemMeta,
    ) {
        state.new_archetype(archetype, &mut system_meta.archetype_component_access);
    }

    #[inline]
    unsafe fn get_param<'w, 's>(
        state: &'s mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'w>,
        change_tick: Tick,
    ) -> Self::Item<'w, 's> {
        state.validate_world(world.id());
        // SAFETY: We have registered all of the query's world accesses,
        // so the caller ensures that `world` has permission to access any
        // world data that the query needs.
        let q = unsafe {
            state.get_single_unchecked_manual(
                world,
                system_meta.last_run,
                change_tick,
            )
        }.unwrap();
        return Player(q);
    }
}
