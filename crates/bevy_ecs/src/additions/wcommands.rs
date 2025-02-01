
use derive_more::Deref;
use derive_more::DerefMut;
use crate::{system::{Commands, ResMut, Resource, SystemParam}, world::World};

/// Rust safety is actually broken here. Don't use `World::commands` somehow while you are using
/// `WCommands`
#[derive(Deref, DerefMut)]
pub struct WCommands<'w,'s> {
    v: Commands<'w,'s>,
}
pub struct Internal;
impl Resource for Internal{}
/// Rust safety is actually broken here. Don't use `World::commands` somehow while you are using
// SAFETY:
unsafe impl<'w> SystemParam for WCommands<'w,'_> {
    type State = ();

    type Item<'world, 'state> = WCommands<'world,'world>;

    fn init_state(world: &mut World, system_meta: &mut crate::system::SystemMeta) -> Self::State {
        let _res_state = <ResMut<'w, Internal> as SystemParam>::init_state(world, system_meta);
        system_meta.set_has_deferred();
    }

    /// # Safety:  world was got from `as_unsafe_world_cell`
    /// that is world in not read only. no parallelism.
    unsafe fn get_param<'world, 'state>(
        _: &'state mut Self::State,
        _: &crate::system::SystemMeta,
        world: crate::world::unsafe_world_cell::UnsafeWorldCell<'world>,
        _: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        // Safety #
        //this world should have been created from &mut World, as there is no parallalism.
        let queue = world.get_raw_command_queue();
        // Safety #
        // queue lives and dies with world.
        let commands = Commands::new_raw_from_entities(queue, world.entities());
        return WCommands{v:commands};
    }

    fn apply(_: &mut Self::State, _: &crate::system::SystemMeta, world: &mut World) {
        world.flush();
    }

    fn queue(_: &mut Self::State, _: &crate::system::SystemMeta, _: crate::world::DeferredWorld) {
        unimplemented!()
    }

}

// impl<'w, 's> WCommands<'w, 's> {
//     pub fn entity(&mut self, entity: Entity) -> EntityCommands {
//         self.v.entity(entity)
//     }
//     pub fn spawn_empty(&mut self) -> EntityCommands {
//         self.v.spawn_empty()
//     }
// }
