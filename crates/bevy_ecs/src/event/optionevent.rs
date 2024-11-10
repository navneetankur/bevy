use crate::world::World;

use super::{run_this_event_system, Event, SystemInput};

pub trait OptionEvent {
    fn run(self, world: &mut World);
}
impl<E: Event> OptionEvent for E
where 
    E: SystemInput<Inner<'static> = E>,
{
    fn run(self, world: &mut World) {
        run_this_event_system::<true, E>(self, world);
    }
}
impl<E: Event> OptionEvent for Option<E>
where 
    E: SystemInput<Inner<'static> = E>,
{
    fn run(self, world: &mut World) {
        if let Some(event) = self {
            run_this_event_system::<true, E>(event, world);
        }
    }
}
impl OptionEvent for (){
    fn run(self, _: &mut World) {}
}
impl<O1: OptionEvent, O2: OptionEvent> OptionEvent for (O1, O2)
{
    fn run(self, world: &mut World) {
        self.0.run(world);
        self.1.run(world);
    }
}
impl<O1: OptionEvent, O2: OptionEvent, O3: OptionEvent> OptionEvent for (O1, O2, O3)
{
    fn run(self, world: &mut World) {
        self.0.run(world);
        self.1.run(world);
        self.2.run(world);
    }
}
impl<O1: OptionEvent, O2: OptionEvent, O3: OptionEvent, O4: OptionEvent> OptionEvent for (O1, O2, O3, O4)
{
    fn run(self, world: &mut World) {
        self.0.run(world);
        self.1.run(world);
        self.2.run(world);
        self.3.run(world);
    }
}
