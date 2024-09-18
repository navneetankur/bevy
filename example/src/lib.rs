#[cfg(test)]
mod test {
use bevy::ecs::{component::Component, system::{Query, RunSystemOnce}, world::World};

#[derive(Component)]
struct C1;
#[test]
fn t1() {
    let mut world = World::new();
    let v = world.run_system_once_with(9, s1);
    world.spawn(C1);
    assert_eq!(v, 9);
    let v = world.run_system_once_with(8, s1);
    assert_eq!(v, 8);
}

fn s1(a: u8, _q: Query<&C1>) -> u8 {
    return a;
}

fn s2(a: u8, _w: &mut World) -> u8 {
    return a;
}

}
