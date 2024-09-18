#[cfg(test)]
mod test {
use bevy::ecs::{component::Component, system::{Query, ResMut, RunSystemOnce}, world::World, Resource};

#[derive(Component, Clone)]
struct C1;

#[derive(Resource, Clone)]
struct R1(u8);

#[test]
fn t1() {
    let mut world = World::new();
    let v = world.run_system_once_with(9, s1);
    world.spawn(C1);
    assert_eq!(v, 9);
    let v = world.run_system_once_with(8, s2);
    assert_eq!(v, 8);
}
fn s1(a: u8, _q: Query<&C1>) -> u8 {
    return a;
}

fn s2(a: u8, _w: &mut World) -> u8 {
    return a;
}

#[test]
fn event_system() {
    let mut world = World::new();
    world.insert_resource(R1(0));
    world.register_event_system(inc_on_c1);
    world.register_event_system(add_it);
    world.send(R1(4));
    let v = world.resource::<R1>();
    assert_eq!(v.0, 5);

}

fn add_it(what: R1, mut to: ResMut<R1>) -> C1 {
    to.0 += what.0;
    return C1;
}

fn inc_on_c1(_: C1, mut to: ResMut<R1>) {
    to.0 += 1;
}

}
