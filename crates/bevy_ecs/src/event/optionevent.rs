use crate::world::World;

use super::{run_this_event_system, Event, SystemInput};

pub trait OptionEvent {
    fn run(self, world: &mut World);
}
impl OptionEvent for (){ fn run(self, _: &mut World) {} }

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
        let Some(event) = self else {return};
        event.run(world);
    }
}
macro_rules! impl_option_event_tuple {
    ($($param: ident),*) => {
        impl<$($param: OptionEvent,)*> OptionEvent for ($($param,)*) {
            fn run(self, world: &mut World) {
                #[allow(non_snake_case)]
                let ($($param,)*) = self;
                $(
                    $param.run(world);
                )*
            }
        }
    }
}

impl_option_event_tuple!(O1);
impl_option_event_tuple!(O1, O2);
impl_option_event_tuple!(O1, O2, O3);
impl_option_event_tuple!(O1, O2, O3, O4);
impl_option_event_tuple!(O1, O2, O3, O4, O5);
