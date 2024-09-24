#[cfg(test)]
mod test {
use bevy::ecs::{component::Component, event::{Event, eventslicer::EventSlicer}, system::{ResMut, Resource}, world::World};

#[derive(Component, Clone, Copy, Event)]
struct C1;

#[derive(Resource, Clone, Copy, Event)]
struct R1(u8);

#[derive(Resource, Event)]
struct R2(u8);
#[test]
fn event_system() {
    let mut world = World::new();
    world.insert_resource(R1(0));
    // world.register_event_system(inc_on_c1);
    world.register_event_system(add_it);
    world.register_event_system(add_it2);
    world.register_event_system(add_it3);
    world.register_event_system(inc_on_c1);
    world.send(R1(4));
    world.send(R2(5));
    let v = world.resource::<R1>();
    assert_eq!(v.0, 17);
}
fn add_it(what: &R1, mut to: ResMut<R1>) -> C1 {
    to.0 += what.0;
    return C1;
}
fn add_it2(what: &R2, mut to: ResMut<R1>) -> C1 {
    to.0 += what.0;
    return C1;
}
fn add_it3(what: R2, mut to: ResMut<R1>) -> C1 {
    to.0 += what.0;
    return C1;
}
fn inc_on_c1(_: C1, mut to: ResMut<R1>) {
    to.0 += 1;
}

#[derive(Event)]
struct E1(u8);

#[test]
fn slicer_event_system() {
    let mut world = World::new();
    world.register_event_system(send_slice_event);
    world.register_event_system(count_in_slice);
    world.send(E1(0));
}

fn send_slice_event(_: E1, mut slicer: EventSlicer<E1>) {
    for i in 0..10 {
        slicer.push(E1(i));
    }
}
fn count_in_slice(events: &[E1]) {
    let mut i = 0;
    for e in events {
        assert_eq!(e.0, i);
        i += 1;
    }
}

}
