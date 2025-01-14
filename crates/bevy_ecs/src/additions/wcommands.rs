use derive_more::derive::{Deref, DerefMut};
use crate::{system::{Commands, ResMut, Resource, SystemParam}, world::{CommandQueue, World}};

#[derive(Deref, DerefMut)]
pub struct WCommands<'w,'s> {
    pub v: Commands<'w,'s>,
}
pub struct Internal {
    queue: Option<CommandQueue>,
    // res_state: ComponentId,
}
impl Resource for Internal{}
unsafe impl<'w,'s> SystemParam for WCommands<'w,'s> {
    type State = Internal;

    type Item<'world, 'state> = WCommands<'world,'state>;

    fn init_state(world: &mut World, system_meta: &mut crate::system::SystemMeta) -> Self::State {
        let _res_state = <ResMut<'w, Internal> as SystemParam>::init_state(world, system_meta);
        return Internal { queue: None,
        // res_state
        };
    }

    /// # Safety:  world was got from as_unsafe_world_cell
    /// that is world in not read only. no parallelism.
    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        _: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        _: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        // Safety: world was got from as_unsafe_world_cell
        let queue = world.world_mut().extras.queue.take().unwrap();
        state.queue = Some(queue);
        let commands = Commands::new_from_entities(state.queue.as_mut().unwrap(), &world.entities());
        let rv = WCommands{v: commands};
        return rv;
    }

    fn apply(state: &mut Self::State, _: &crate::system::SystemMeta, world: &mut World) {
        let Some(mut queue) = state.queue.take() else {return};
        queue.apply(world);
        debug_assert!(world.extras.queue.is_none());
        world.extras.queue = Some(queue);
    }

    fn queue(_: &mut Self::State, _: &crate::system::SystemMeta, _: crate::world::DeferredWorld) {
        unimplemented!()
    }

//     unsafe fn validate_param(
//         state: &Self::State,
//         system_meta: &crate::system::SystemMeta,
//         world: crate::world::unsafe_world_cell::UnsafeWorldCell,
//     ) -> bool {
//         <ResMut<'w, Internal> as SystemParam>::validate_param(&state.res_state, system_meta, world)
//     }
}
