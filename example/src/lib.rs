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
#[derive(Event)]
struct E2(u8);

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
#[test]
fn slicer_forward_event_system() {
    let mut world = World::new();
    world.register_event_system(send_slice_event_forwarding);
    world.register_event_system(e1_call_counter1);
    world.init_resource::<Counter>();
    world.send(E2(0));
    let counter = world.resource::<Counter>();
    assert_eq!(counter.0, 10);
}
fn send_slice_event_forwarding(_: E2, mut slicer: EventSlicer<E1, true>) {
    for i in 0..10 {
        slicer.push(E1(i));
    }
}
#[derive(Resource, Default)]
struct Counter(u8);
fn e1_call_counter1(_: &E1, mut count: ResMut<Counter>) {
    count.0 += 1;
}

#[test]
fn exclusive_systems_test() {
    let mut world = World::new();
    world.init_resource::<Counter>();
    world.register_event_system(iam_exclusive);
    world.send(E1(0));
    let c = world.resource::<Counter>();
    assert_eq!(c.0, 14);
}
fn iam_exclusive(e1: &E1, w: &mut World) {
    let mut c = w.resource_mut::<Counter>();
    c.0 = 14;
}

}
