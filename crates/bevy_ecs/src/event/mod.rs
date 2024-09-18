use crate::{component::Component, traversal::Traversal};
pub trait Event: Component {
    /// The component that describes which Entity to propagate this event to next, when [propagation] is enabled.
    ///
    /// [propagation]: crate::observer::Trigger::propagate
    type Traversal: Traversal;

    /// When true, this event will always attempt to propagate when [triggered], without requiring a call
    /// to [`Trigger::propagate`].
    ///
    /// [triggered]: crate::system::Commands::trigger_targets
    /// [`Trigger::propagate`]: crate::observer::Trigger::propagate
    const AUTO_PROPAGATE: bool = false;
}
